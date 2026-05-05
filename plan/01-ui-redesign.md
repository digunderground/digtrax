# Phase 1 — UI redesign

## Goal

Replace the current top-tab + modal-dialog UI shell with a sidebar-driven, content-first layout that puts Quick Tag at the center of the experience. Aesthetic targets: Traktor Pro 4 (dense professional DJ tooling, color-coded tracks, prominent waveforms, panel-based layout) and 2026 Apple design language (continuous-curvature corners, layered translucent surfaces, generous whitespace, semantic color tokens, light + dark theming, system typography).

## What's wrong with the current UI

These are the concrete problems we're fixing — every change in this plan should map back to one of them:

1. **Quick Tag is the most important feature but doesn't get visual priority.** The default route `/` is [client/src/views/Index.vue](../client/src/views/Index.vue) — a marketing card grid, not a workspace. First-time and returning users both land in the same screen and have to navigate to Quick Tag every session.
2. **Top-tab navigation wastes vertical space and is shallow.** All seven features sit in a single `q-tabs` row in [client/src/App.vue](../client/src/App.vue). No room for sub-navigation, recent paths, library shortcuts, or status. A sidebar gives all of these for free.
3. **Settings is a 1000+ line monolith inside a fixed-size dialog.** [client/src/components/Settings.vue](../client/src/components/Settings.vue) crams three tabs (`quicktag`, `quicktag-custom`, `advanced`) into one component with 20+ inline `ref()`s, no sub-component split, and a Quasar dialog that doesn't grow with the viewport. On a 27" display it's a postage stamp.
4. **No design system.** Colors live partly in [client/src/style/quasar.scss](../client/src/style/quasar.scss) Quasar SCSS vars, partly inline. No spacing scale, no elevation tokens, no semantic role colors (success-bg vs success-fg vs success-border). Consistency is impossible.
5. **Dark-only, hard-coded.** `App.vue` calls `$q.dark.set(true)` in `onMounted`. There's a primary-color picker but no theme switcher.
6. **Inconsistent component density and spacing.** Some routes use `q-pa-lg`, some use `q-pa-md`, some use ad-hoc inline padding. Tiles in Quick Tag don't match cards on the homepage don't match panels in Tag Editor.

## Architectural target

```
┌─────────────────────────────────────────────────────────────────┐
│ Title bar (drag region)                  ⌘+K  ⚙  light/dark │
├──────────┬──────────────────────────────────────────────────────┤
│          │                                                      │
│ Sidebar  │  Main content (route view)                          │
│ ───────  │                                                      │
│ Library  │                                                      │
│  Recent  │                                                      │
│  Folders │                                                      │
│          │                                                      │
│ Tools    │                                                      │
│  Quick   │  (active route — Quick Tag by default)              │
│   Tag ●  │                                                      │
│  Tag     │                                                      │
│   Editor │                                                      │
│  Auto    │                                                      │
│   Tag    │                                                      │
│  Audio   │                                                      │
│   Feat.  │                                                      │
│  Renamer │                                                      │
│          │                                                      │
│ Help     │                                                      │
└──────────┴──────────────────────────────────────────────────────┘
│ Player bar (when track playing)                                 │
└─────────────────────────────────────────────────────────────────┘
```

Within Quick Tag specifically, keep its existing internal three-pane structure (file browser ◇ track grid ◇ inspector) but unify the surface treatment — same panel chrome, same separator style, same scroll behavior — across all routes.

## Design system foundation (do this first)

This is the unblocker for everything else. Land it before touching individual views.

### 1. Token layer

Replace [client/src/style/quasar.scss](../client/src/style/quasar.scss) and [client/src/style/app.scss](../client/src/style/app.scss) with a tokenized system:

- `client/src/style/tokens.scss` (new) — single source of truth, CSS custom properties:
  - **Color**: semantic roles only (`--color-bg`, `--color-bg-elevated`, `--color-bg-overlay`, `--color-fg`, `--color-fg-muted`, `--color-fg-subtle`, `--color-accent`, `--color-accent-hover`, `--color-border`, `--color-border-strong`, `--color-success`, `--color-warning`, `--color-danger`, plus `-bg` / `-fg` / `-border` variants for status colors). Two themes — `:root` is dark (default), `[data-theme="light"]` is light. Keep current teal `#00D2BF` as accent in both.
  - **Spacing**: `--space-1` through `--space-9` on a 4-px grid (4, 8, 12, 16, 20, 24, 32, 48, 64).
  - **Radius**: `--radius-xs` 4, `--radius-sm` 6, `--radius-md` 10, `--radius-lg` 14, `--radius-xl` 20 — continuous-curvature feel, larger than current Quasar defaults.
  - **Elevation**: `--shadow-1` … `--shadow-4` (subtle, layered — dark theme uses shadow + border, light theme uses shadow only).
  - **Typography**: `--font-ui` (system stack: `-apple-system, "SF Pro Text", "Segoe UI Variable", system-ui, sans-serif`), `--font-display` (keep Dosis for branded headers), `--font-mono` (system mono). Sizes `--text-xs` 11 → `--text-3xl` 28.
  - **Motion**: `--ease-standard` `cubic-bezier(.2,.8,.2,1)`, `--duration-fast` 120ms, `--duration-base` 200ms, `--duration-slow` 320ms.
- Wire Quasar SCSS vars in [client/src/style/quasar.scss](../client/src/style/quasar.scss) to **read from CSS vars**, not hard-coded hexes — Quasar reads at compile time, so set `$primary: #00D2BF` etc. but also expose them as `--color-accent` so runtime theming works.
- Add a thin `theme.ts` helper in [client/src/scripts/](../client/src/scripts/) that toggles `data-theme` on `<html>` and persists to settings.

### 2. Component primitives

New components in `client/src/components/ui/` that wrap Quasar with our tokens:

- `Panel.vue` — replaces ad-hoc `<div class="bg-dark q-pa-md">` patterns. Variants: `default`, `inset`, `floating`. Always uses tokenized bg/border/radius.
- `Toolbar.vue` — top-of-panel header with title slot, actions slot, consistent height.
- `Sidebar.vue`, `SidebarSection.vue`, `SidebarItem.vue` — for the new shell.
- `FieldRow.vue` — label + input + help-text combo, replaces the 50+ ad-hoc field layouts in Settings.
- `Stat.vue` — small KPI / count display for the sidebar (e.g. "238 files").

Existing Quick Tag, Tag Editor, etc. components are NOT rewritten — they re-skin by adopting these primitives over time. This phase requires only that the primitives exist and are used in the new shell + the redesigned Quick Tag and Settings.

### 3. Theme switcher

- Add a top-right control in the new title bar: light / dark / system.
- Store choice in `Settings` (extend the Rust `Settings` struct in [crates/onetagger-shared/src/lib.rs](../crates/onetagger-shared/src/lib.rs); serde will round-trip it). Default `system`.
- On boot, read `prefers-color-scheme` for `system` mode; listen for changes.

## PR-sized step plan

### Step 1 — Tokens + primitives (no visible change)
- Add `tokens.scss`, refactor existing styles to use token vars where trivial, no layout changes.
- Add `Panel`, `Toolbar`, `FieldRow` primitives, unused at first.
- Verify: `pnpm run build` clean, app launches, looks identical.

### Step 2 — Theme switcher + light mode
- Wire `data-theme` toggle, add light-mode token values.
- Add theme picker in the Settings dialog (still old layout).
- Verify: toggle in Settings actually changes the appearance live; preference persists across restarts.

### Step 3 — New shell layout (App.vue rewrite)
- Replace top `q-tabs` with `Sidebar`.
- Make the sidebar collapsible (icons-only at narrow widths, full at wide).
- Move the Settings gear into a sidebar bottom-anchor (`SidebarItem` style) and into a new `⌘+,` shortcut.
- Title bar with traffic-light region on macOS (CSS `-webkit-app-region: drag` reserved area).
- Update [client/src/scripts/router.ts](../client/src/scripts/router.ts): change `/` from `Index.vue` to redirect to `/quicktag`. **Keep `Index.vue` reachable as `/welcome`** for first-launch onboarding, but no longer the default.
- First-launch detection: if `Settings.path` is empty, show a one-time welcome state inside Quick Tag itself ("Drag a folder to get started") — not a separate route. Removes the marketing-screen detour.
- Verify: every existing route still renders; sidebar highlights the active route; collapsing drawer behavior is unchanged for Quick Tag (left file browser, right inspector).

### Step 4 — Quick Tag re-skin (no behavior change)
- Adopt `Panel` / `Toolbar` primitives in [client/src/views/QuickTag.vue](../client/src/views/QuickTag.vue), [client/src/components/QuickTagFileBrowser.vue](../client/src/components/QuickTagFileBrowser.vue), [client/src/components/QuickTagTile.vue](../client/src/components/QuickTagTile.vue), [client/src/components/QuickTagTileThin.vue](../client/src/components/QuickTagTileThin.vue), [client/src/components/QuickTagRight.vue](../client/src/components/QuickTagRight.vue).
- Tile design: larger artwork, color-coded energy/mood strip down the side (Traktor track-color cue), denser metadata block. Selection state uses accent border + bg tint, not just outline.
- Keyboard layout indicator: persistent legend at the bottom of the right inspector showing the user's current mood / genre / energy bindings — currently you have to remember them.
- File browser: tree view with folder icons, file counts per folder (`Stat` primitive), recently-opened pinning at the top.
- Player bar: rework as a single full-width bar with waveform, transport, current track metadata. Already-existing PlayerBar component, just re-skinned.
- Verify: every keyboard shortcut, drag-drop, multi-select pattern still works. Compare side-by-side with the old build.

### Step 5 — Settings redesign
- Convert [client/src/components/Settings.vue](../client/src/components/Settings.vue) from `<q-dialog>` to a real route at `/settings` (still openable via a header gear icon and `⌘+,`, but it's a navigated view, not a modal).
- Layout: macOS System Settings 2023+ pattern — left rail with categories, right pane with content. Sizes with the viewport (min-width 720, no max).
- Split the monolith into sub-components per category, each in `client/src/components/settings/`:
  - `LibraryPaths.vue` — input folder, recursive toggle, recent paths
  - `QuickTagConfig.vue` — energy bindings, mood frames, mood values, genres
  - `CustomTags.vue` — current `quicktag-custom` tab
  - `Platforms.vue` — Beatport/Discogs/Spotify auth (currently scattered)
  - `Appearance.vue` — theme, accent color, density, font choice
  - `Keybindings.vue` — global shortcuts (currently buried)
  - `Advanced.vue` — debug, log paths, version
- Each sub-component owns its own `ref()`s and reads/writes `$1t.settings`. The shell only handles category routing.
- Verify: all settings round-trip to disk identically (binary-diff the persisted config before/after); no setting becomes unreachable.

### Step 6 — Polish pass
- Loading states, empty states, error states for every panel — currently inconsistent (some show spinners, some flash empty, some show nothing).
- Motion: 200ms ease for panel mounts, sidebar collapse, theme toggle.
- Accessibility: focus rings using accent color, keyboard navigation across the sidebar, `aria-current` on active route, dialog trap-focus where modals remain (folder browser etc.).
- Density toggle in Appearance: comfortable / compact (affects tile size, row height, font sizes via tokens).

## Deferred — Auto Tag follow-ups

- **"Need more platforms?" / Platforms Repository UI.** Currently hidden behind a `SHOW_PLATFORMS_REPO = false` feature flag in [client/src/components/AutotaggerPlatforms.vue](../client/src/components/AutotaggerPlatforms.vue) — markup is preserved in a `v-if` block so reinstatement is a one-line flag flip. The original UI is a spec dump (heading + paragraph + button + doc link) that doesn't fit the V4 cards aesthetic. When we reinstate:
    - Move the Platforms Repository launcher into the Auto Tag toolbar (next to the run/CLI buttons), or its own slim card under Select Platforms titled "Extend platforms".
    - Reduce the ceremony: a single "Browse platforms…" button + small help link is enough; the install dialog ([client/src/components/PlatformsRepo.vue](../client/src/components/PlatformsRepo.vue)) already carries the explanatory copy.
    - Verify the install flow still works on macOS / Windows / Linux after the IO changes that landed in PR-1B's protocol break.

- **Audio Features.** Hidden behind a `SHOW_AUDIO_FEATURES = false` feature flag in [client/src/components/DigTraxNavRail.vue](../client/src/components/DigTraxNavRail.vue) — only the sidebar nav entry is removed; the route at `/audiofeatures` still resolves to [client/src/views/AudioFeatures.vue](../client/src/views/AudioFeatures.vue) so any deep links / scheduled jobs continue to work. When we reinstate:
    - Redesign the Audio Features view to match the V4 cards aesthetic (it's currently still the OneTagger-era layout).
    - Decide whether Spotify auth lives here or moves into Settings → Platforms (it's used by both Auto Tag and Audio Features today).
    - Confirm the current backend flow ([crates/digtrax-autotag/src/audiofeatures.rs](../crates/digtrax-autotag/src/audiofeatures.rs)) still matches the UI surface after any shape changes.

## Risks and open questions

- **Wry/webkit2gtk parity.** macOS uses WebKit, Windows uses WebView2 (Chromium-based), Linux uses webkit2gtk. CSS features like `backdrop-filter`, `:has()`, container queries are supported on all three modern engines but webkit2gtk's age varies by distro. Test on Linux before relying on the newest CSS features. If we hit a parity gap, fall back to non-translucent panel surfaces.
- **Dosis vs system font.** Dosis is part of the brand. Plan keeps it for `--font-display` (logo, page titles) and switches body to system. Open question: do we want Dosis on all headers or only on the app-name lockup? Recommend system for headers too — it's denser and reads better at small sizes.
- **Macros and color picker on settings.** Existing Settings has a hand-rolled Quasar color-name list for moods/genres. Replace with our own swatch grid driven by the token palette so it stays consistent across themes.
- **Persisted-state migration.** Adding `theme`, `density`, `accentColor` to `Settings` is additive — old configs deserialize fine if the fields use `#[serde(default)]`. Verify on the first load with a pre-existing config file.
- **First-launch UX.** Replacing the welcome screen with in-Quick-Tag empty state is a judgment call — the marketing screen does serve as a "what is this app" moment. Mitigation: keep `/welcome` reachable from the sidebar's Help section.

## Verification

After each step:
1. `cd client && pnpm run build` — no type or build errors.
2. `cargo build --release` — all crates compile.
3. `cargo bundle --release` — `.app` builds.
4. Launch `target/release/bundle/osx/OneTagger.app`, exercise the changed surface manually.
5. Diff a saved config file before/after Settings round-trip — must be byte-identical for unchanged sections.

End-of-phase acceptance:
- App launches directly into Quick Tag.
- Sidebar navigation replaces top tabs across every route.
- Light + dark themes both look polished and switch live.
- Settings is a full route, scales with the window, broken into sub-components.
- All keyboard shortcuts, drag-drop, multi-select behaviors that worked before still work identically.
- No new native deps in `Cargo.toml`. No major version bumps in `client/package.json`.
