# Phase 2 — Tag Editor: bulk edit, filter-by-tag, custom-tag rename across files

## Goal

Turn the Tag Editor from a one-file-at-a-time form into a true library tool. Users should be able to:

1. Select N files in a folder (manually, or by filtering on tag values).
2. Edit a field once and have the change written to all selected files.
3. Rename a custom tag (the frame name itself, e.g. `MYFRAME` → `MOOD_NEW`) across every selected file in one action.
4. See per-file progress and any failures during a batch write — without the UI freezing.

## Why this is currently impossible

Source: read alongside [client/src/views/TagEditor.vue](../client/src/views/TagEditor.vue), [crates/onetagger-ui/src/socket.rs](../crates/onetagger-ui/src/socket.rs), [crates/onetagger-ui/src/tageditor.rs](../crates/onetagger-ui/src/tageditor.rs), [crates/onetagger-tag/src/lib.rs](../crates/onetagger-tag/src/lib.rs).

| Layer | Single-file assumption | File / line |
|---|---|---|
| Frontend | `loadFile(path)` loads one file; `file.value` holds one tag set; `changes[]` array is per-file; `save()` ships one path | [TagEditor.vue:405](../client/src/views/TagEditor.vue), `file.value.path` checks at line 413, save flow ~ line 525 |
| WebSocket | `TagEditorLoad { path }`, `TagEditorSave { changes: TagChanges }` — one path per message | [socket.rs:494-519](../crates/onetagger-ui/src/socket.rs) |
| Tag I/O | `TagChanges { path: PathBuf, changes: Vec<TagChange> }`. `commit()` opens one file, applies, saves | [crates/onetagger-tag/src/lib.rs:172](../crates/onetagger-tag/src/lib.rs) |

The user's specific complaint — "if I change a custom tag title I need to manually update every file one-by-one" — is the visible symptom of all three layers being hard-wired to single-file. There's no "rename tag X to Y across these files" verb anywhere in the system.

## Architectural changes (data model + protocol)

### Backend — tag layer

Two new methods on `TagChanges` in [crates/onetagger-tag/src/lib.rs](../crates/onetagger-tag/src/lib.rs):

```rust
impl TagChanges {
    /// Apply all changes to N files. Streams per-file results via the callback so the
    /// caller (the WS handler) can forward progress to the UI without buffering the whole
    /// batch in memory. Continues on per-file errors; returns aggregate status.
    pub fn commit_multi<F>(
        &self,
        paths: &[PathBuf],
        on_progress: F,
    ) -> Result<BatchReport, anyhow::Error>
    where
        F: FnMut(&Path, Result<(), anyhow::Error>);
}

/// New change variant — rename a frame across formats.
pub enum TagChange {
    // ... existing variants ...
    /// Rename a tag frame from `from` to `to`. For ID3 raw frames, both are frame IDs
    /// (e.g. `TXXX:OLD` → `TXXX:NEW`). For Vorbis, lowercase keys. For MP4, atom names.
    /// The implementation reads the existing value, writes it under the new key, and
    /// deletes the old key.
    RenameFrame { from: String, to: String },
}
```

`BatchReport` is a small struct: `total`, `succeeded`, `failed: Vec<(PathBuf, String)>`.

Why a callback rather than collecting into a `Vec<Result>`: a 5,000-file library batch should stream UI updates as each file lands, not block on the whole set. This mirrors how the autotagger already streams progress over the WS — same pattern, same UX.

### Backend — WebSocket protocol

In [crates/onetagger-ui/src/socket.rs](../crates/onetagger-ui/src/socket.rs), add three new actions and **replace** the existing single-file save action (no shim — see README hard constraints):

```rust
// REPLACED: TagEditorSave is now plural-by-default
TagEditorSave { paths: Vec<PathBuf>, changes: Vec<TagChange> }
//                                  ^ note: just the changes, the per-file path
//                                    moves up to the request envelope

// NEW
TagEditorFilter { folder: PathBuf, recursive: bool, query: TagFilterQuery }
TagEditorBulkProgress  // server → client, emitted during a bulk save
```

`TagFilterQuery` supports composable predicates:

```rust
pub enum TagFilterQuery {
    /// Tag exists at all
    HasTag { tag: String },
    /// Tag value matches (case-insensitive contains, by default)
    Equals { tag: String, value: String, mode: MatchMode },
    /// Tag is missing or empty
    Missing { tag: String },
    /// All sub-queries must match
    And(Vec<TagFilterQuery>),
    /// Any sub-query matches
    Or(Vec<TagFilterQuery>),
}

pub enum MatchMode { Exact, Contains, Regex }
```

Filter handler walks the folder (reuse `AudioFileInfo::load_files_iter` — already used by autotagger), opens each file's tags read-only, evaluates the query, returns the list of matching paths. **Returns paths only**, not full tag dumps — the UI then loads the union of editable fields across the matched set with a follow-up `TagEditorLoad` (extended to accept `Vec<PathBuf>`, see below).

`TagEditorLoad` is extended to take `paths: Vec<PathBuf>` and return:

```rust
pub struct LoadedFileSet {
    pub files: Vec<LoadedFile>,
    /// Union of tag keys present across all files, with per-key statistics so the UI
    /// can render "Multiple values" / "All same" / "Some have it".
    pub field_summary: Vec<FieldSummary>,
}

pub struct FieldSummary {
    pub tag: String,
    /// How many of the loaded files have this tag.
    pub present_in: usize,
    /// If all present files share one value, this is Some(that value). Otherwise None.
    pub common_value: Option<String>,
}
```

Single-file load = `Vec` of one — no special case in the protocol.

### Frontend — TagEditor.vue rewrite

The view splits into three panes (consistent with the redesigned UI shell from Phase 1):

```
┌────────────────────────┬──────────────────────────┬──────────────────────┐
│ File list / filters    │ Selected files (N)       │ Tag editor           │
│  - Folder browser      │  - Multi-select grid     │  - Tabs: Common /    │
│  - Filter bar          │  - Sortable columns      │    Custom / Raw      │
│  - "Select all"        │  - Selection counter     │  - Per-field "apply  │
│  - Saved filters       │  - Drag to "your list"   │    to all" affordance│
│                        │                          │  - Rename custom tag │
│                        │                          │    action            │
└────────────────────────┴──────────────────────────┴──────────────────────┘
```

Key new behaviors:

- **Multi-select**: checkboxes + Shift+Click range + ⌘+A. Live count in the toolbar ("12 of 238 selected").
- **Filter bar**: structured input — `field` `operator` `value`, with chips for stacked filters. Behind the scenes builds a `TagFilterQuery`. Saved filters live in `Settings`.
- **Field summary in the editor pane**: each field shows its `FieldSummary`. If `common_value` is `Some`, the input shows that value plainly. If `None`, the input shows a "Multiple values — type to overwrite" placeholder with a small "show breakdown" disclosure that lists the distinct values and their file counts.
- **Apply-to-all toggle**: lives next to each editable field. Default-on when N>1. When off, the field is read-only ("change applies to selected files only when this is on").
- **Rename custom tag**: a dedicated action in the Custom tab — opens a small dialog with `from` and `to` fields, shows preview ("23 files have `MYFRAME`, will become `MOOD_NEW`"), confirms, and dispatches a `RenameFrame` change against the selection.
- **Progress drawer**: bottom-of-view drawer that opens when a bulk save starts. Streams `TagEditorBulkProgress` events: per-file success/failure, running totals, ability to cancel mid-batch (sends a `TagEditorCancel` WS message; backend checks a flag between files).

State management: do NOT try to keep all loaded files reactive in memory. Hold the **selection paths** plus the **field summary**; load a single file's full tag detail on demand for the "Raw" tab. This keeps a 5k-file selection from murdering the renderer.

## PR-sized step plan

### Step 1 — Backend: batch commit + filter, no UI
- Add `TagChanges::commit_multi` and `TagChange::RenameFrame` to [crates/onetagger-tag/src/lib.rs](../crates/onetagger-tag/src/lib.rs).
- Add the WS messages: `TagEditorFilter`, extend `TagEditorLoad`/`TagEditorSave` to take `Vec<PathBuf>`. Remove the old single-file shape.
- `#[cfg(test)]` tests for `commit_multi`: tmpdir + small synthesized files (mp3, flac, m4a) + verify writes succeed and partial failures are reported correctly.
- `#[cfg(test)]` tests for the filter query evaluator: covers `Equals`/`HasTag`/`Missing`/`And`/`Or`/`Regex`.
- Frontend stays on the old single-file flow during this step — the WS protocol breaks but the old UI still calls the single-file shapes; we land Step 2 in the same PR or as the very next PR to keep `master` shippable. **In practice land Step 1 + Step 2 together.**

### Step 2 — Frontend: multi-file load and edit, no filter UI yet
- Rewrite [client/src/views/TagEditor.vue](../client/src/views/TagEditor.vue) to hold a `selectedPaths: Ref<string[]>` instead of a single `path`.
- Implement the three-pane layout using the Phase 1 primitives.
- Implement `FieldSummary` rendering, "apply to all" toggles, multi-file save dispatch.
- Re-implement the existing single-file experience as a degenerate case (`selectedPaths.length === 1`) — no separate code path.
- Verify: open a folder, multi-select 10 files, change `Genre` to `House`, save, confirm via a separate tool (e.g. `mp3info`, `mid3v2`, or just reload in OneTagger) that all 10 files have the new genre.

### Step 3 — Filter UI
- Filter bar component with structured `field/operator/value` input + chip stack.
- Wire to `TagEditorFilter` WS message.
- Saved filters in `Settings.tagEditor.savedFilters: Vec<NamedFilter>` (additive serde field).
- Verify: filter `Genre = "House"` in a 1k-file folder under 2 seconds; filter result matches manual inspection of a known sample.

### Step 4 — Custom tag rename + progress drawer
- "Rename custom tag" dialog in the Custom tab.
- Bottom progress drawer that subscribes to `TagEditorBulkProgress` events.
- Cancel button wired to a `TagEditorCancel` message; backend checks an `AtomicBool` between files.
- Verify: rename `MYFRAME` → `MOOD_NEW` across 50 files; confirm the old frame is gone and the new one is present; confirm cancellation mid-way actually stops within one file's worth of work.

## Edge cases to handle explicitly

- **Format mismatch on rename.** Renaming `TXXX:OLD` → `TXXX:NEW` is well-defined for ID3 but not for Vorbis (there's no namespacing). For each format, the `RenameFrame` impl must error out with a clear message rather than silently doing nothing.
- **Read-only files / permission errors.** Surface in the per-file failure list, do not abort the batch.
- **Concurrent modification.** If a file is touched by another process between filter and save, the save should report a clear stale-read error for that file.
- **Atomicity.** Per-file writes are atomic at the format-library level (lofty / id3 / metaflac / mp4ameta all write via temp + rename). Batch is not atomic across files — the progress drawer makes this explicit. Document in the dialog ("partial completion possible — check the report").
- **Selection persistence.** When a filter changes, current selection is cleared with a confirmation if there were unsaved changes. Don't silently lose work.
- **Pictures.** Bulk-applying an embedded cover image is in scope. Bulk-removing all pictures is in scope. Bulk-changing one picture per file (different art per file) is explicitly out of scope.

## Open questions for the user

These are intentional choices that need a decision before / during Step 2. Surfaced here so they don't sneak in as defaults:

1. **Default match mode for filters.** `Equals` semantics — exact, contains, or regex by default? Recommend `Contains` (case-insensitive).
2. **"Apply to all" default state when N>1.** On or off? Recommend on; the whole point of multi-select is to write to all.
3. **Field summary disclosure.** Always visible or click-to-expand? Recommend click-to-expand (cleaner editor pane).
4. **What counts as a "common" tag?** Currently: built-in tags (artist, title, album, year, genre, bpm, key, ratings) + custom tags whose frame names appear in any of the selected files. Recommend that exact union; surface custom tags that are present in some-but-not-all files in a separate "Sometimes present" group with a count.
5. **Bulk save commit speed.** Tag library writes synchronously per file. For a 5k-file save we'd see ~minutes of wall time. Acceptable with the progress drawer? Or do we need parallel writes (rayon)? Recommend ship serial first, measure, parallelize only if it's a real complaint.

## Verification

End-to-end manual test (post-Step 4):

1. Point Tag Editor at a 200-file test folder.
2. Apply filter `Genre = "Trance"`. Get N matches.
3. Multi-select all matches. Change `Energy` to `4` and `Comment` to `Reviewed 2026`. Save.
4. Confirm the progress drawer shows N successes, no failures.
5. Reload one of the files via single-file selection. Confirm the new values landed.
6. Rename custom tag `INITIALKEY` → `STARTKEY` across the same selection. Save.
7. Confirm via the Custom tab that `STARTKEY` is present and `INITIALKEY` is gone.

Automated test floor:
- `commit_multi` integration test against a tmpdir of synthesized files for each format (id3 mp3, vorbis flac, mp4 m4a) — write three different changes, assert all three landed.
- `RenameFrame` test per format — assert old frame absent, new frame present with original value.
- Filter query evaluator unit tests — `And`/`Or`/`Regex`/`Missing` correctness on synthesized tag maps.

## Acceptance for Phase 2

- Tag Editor supports multi-file selection with checkbox / range / select-all.
- Tag Editor supports filtering by tag value and showing the result set.
- Tag Editor edits write to all selected files in a single user action.
- Custom tag rename works across N files in one action.
- Progress drawer shows per-file results and supports cancel.
- All single-file behaviors that worked before still work identically (degenerate case of N=1).
- Existing platform plugins continue to load (no ABI breakage).
