# OneTagger design system — review

Three directional variants for the Phase 1 UI redesign. Open each `preview.html` in your browser to see the system applied to real OneTagger screens (Quick Tag, Tag Editor, Settings). Read each `DESIGN.md` for the spec.

| # | Direction | Vibe | File |
|---|---|---|---|
| 01 | **Studio Apple Polish** | Refined dark mode + Apple polish. Continuous-curvature corners, translucent panels, Geist + Dosis. Both light & dark. | [01-studio-polish/](01-studio-polish/) |
| 02 | **Studio Brutalist** | Raw, mono-everywhere, function-only. JetBrains Mono, hard rectangles, phosphor-green accent. Dark only. | [02-studio-brutalist/](02-studio-brutalist/) |
| 03 | **Hot Cue Maximalist** | Peak-hour club energy. Heavy color, glow halos, vibrancy panels. Space Grotesk display, Geist body. | [03-hot-cue-maximalist/](03-hot-cue-maximalist/) |
| 04 | **DigTrax Streamlined** ★ | V3 colors + V2 mono typography. Brand renamed to DigTrax. Quick Tag has Rows ↔ Cards toggle (Rows default). | [04-digtrax-streamlined/](04-digtrax-streamlined/) |

## How to review

```sh
# Open all four at once:
open design/01-studio-polish/preview.html design/02-studio-brutalist/preview.html design/03-hot-cue-maximalist/preview.html design/04-digtrax-streamlined/preview.html
```

For each preview, evaluate:
1. **Quick Tag layout** — does the file browser / track grid / inspector / player feel right? Quick Tag is the priority feature.
2. **Tag Editor bulk-edit pane** — does the three-pane (filter / selection / editor) layout read clearly?
3. **Settings as a route** — does it feel like a modern Apple-style settings, scaled to viewport?
4. **Type + color hierarchy** — do important things look important? Is data legible?
5. **Density** — too tight? too airy? appropriate for an audio-tool?

After reviewing, tell me which variant to proceed with (or which elements to remix across variants — e.g., "Variant 1 layout, Variant 3 colors, Variant 2 mono for data tables"). I write the canonical `DESIGN.md` at the repo root and we move into Phase 1 Step 1 of [plan/01-ui-redesign.md](../plan/01-ui-redesign.md).

## Folder structure

```
design/
├── README.md                          # this file
├── 01-studio-polish/
│   ├── DESIGN.md                      # full spec for variant 1
│   └── preview.html                   # browser preview, self-contained
├── 02-studio-brutalist/
│   ├── DESIGN.md
│   └── preview.html
└── 03-hot-cue-maximalist/
    ├── DESIGN.md
    └── preview.html
```

After a variant is approved, the canonical `DESIGN.md` is written to the repo root (referenced by [CLAUDE.md](../CLAUDE.md) and [plan/01-ui-redesign.md](../plan/01-ui-redesign.md)). The `design/` folder stays as the source of the exploration.

## Constraints all three variants respect

- Existing accent **#00D2BF teal** is preserved in some form (kept literal in V1 & V3, repurposed as a token color in V2).
- **Dosis** font stays available — kept as display in V1 and V3, dropped from V2 since V2 is mono-only.
- No major dep bumps in [client/package.json](../client/package.json) — variants assume Vite 6 / Quasar 2 / Vue 3.5 stay pinned.
- All variants render correctly inside `wry` (WebKit on macOS, WebView2 on Windows, webkit2gtk on Linux). Where a variant uses `backdrop-filter` it provides a solid-surface fallback.
- All three respect the 8-color **track-color palette** convention from Phase 1 (energy / mood / custom-tag color coding).
