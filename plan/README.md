# OneTagger redesign — plan index

Two phases. Each is independent and shippable on its own; do them in order so the new UI shell is in place before we layer bulk-edit affordances onto the Tag Editor.

| Phase | File | Scope | Approx. effort |
|---|---|---|---|
| 0 | [00-mac-universal-builds.md](00-mac-universal-builds.md) | CI flip from Intel-only to universal (Intel + Apple Silicon) macOS builds. No source changes — workflow YAML + lipo. | 1 PR, 1–2 hours |
| 1 | [01-ui-redesign.md](01-ui-redesign.md) | Full UI shell redesign — sidebar nav, Quick Tag as homepage, redesigned Settings, design-system tokens, light/dark themes, Traktor Pro 4 + Apple 2026 aesthetic | 4–6 PRs over ~2 weeks |
| 2 | [02-tag-editor-bulk-edit.md](02-tag-editor-bulk-edit.md) | Tag Editor: multi-file selection, filter-by-tag-value, bulk write, custom-tag-rename across N files, batch progress streaming | 2–3 PRs over ~1 week |

## Hard constraints both phases must respect

1. **Frontend must build before `cargo build`.** [crates/onetagger-ui/src/lib.rs](../crates/onetagger-ui/src/lib.rs) embeds `client/dist/` at compile time via `include_dir!`. Any new component lands in the binary only after `pnpm run build`.
2. **No major-version dep bumps.** The git history (`30efd81 Fix vite version`, `2429a83 Revert CI version`, `81be41f Fix deps update breakage`) shows repeated burns from dep updates. Vite stays on 6.x, Quasar on 2.x, Vue on 3.5, `wry`/`tao` on their current pins.
3. **No new native deps.** `muda` and `native-dialog` for macOS are already enough; do not introduce additional Tauri/Electron-style deps. The webview shell stays wry/tao.
4. **WebSocket is the API surface.** All UI ↔ backend RPC for Tag Editor changes flows through the existing `/ws` route (see [crates/onetagger-ui/src/socket.rs](../crates/onetagger-ui/src/socket.rs)). Don't add REST endpoints for editor actions.
5. **No backwards-compat shims.** OneTagger ships as a single binary — old clients can't talk to new servers. When we change a WS message shape, change it everywhere in one PR.

## Test bar

OneTagger has no automated test suite (`cargo test` produces zero tests across all 11 crates). Every change is verified by:
- `pnpm run build` — frontend type-checks and bundles cleanly
- `cargo build --release` — all 11 crates compile
- `cargo bundle --release` — `OneTagger.app` builds
- Manual smoke test: launch the `.app`, exercise the changed feature end-to-end

If during Phase 1 or 2 we introduce non-trivial Rust logic (e.g., the bulk-commit batching), add `#[cfg(test)] mod tests` inline in that module — the lack of an existing suite is not a license to ship untested batching logic over real audio files.

## Out of scope (deferred)

- Adding new tagging platforms
- Audio Features algorithmic changes
- Renamer UX (separate redesign turn)
- Auto-tagger UX beyond what's needed for sidebar consistency
- Plugin (`.dylib`/`.so`) ABI changes — existing platform plugins must continue to load
