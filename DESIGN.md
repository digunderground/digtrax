# Variant 04 — DigTrax · Streamlined

## Visual thesis

V3's club-energy color language meets V2's terminal-grade typography. Take Hot Cue Maximalist's saturated track colors, glow halos, and translucent vibrancy panels — then strip every non-mono typeface out and adopt the bracket-and-uppercase typography rules from Studio Brutalist. The result reads simultaneously as performance tool (color, motion, depth) and power tool (mono everywhere, no decorative type, bracketed labels). And the app gets a name that sounds like it: **DigTrax** — DJ slang for crate-digging, monoscript for power-user identity.

**Vibe:** the booth at peak hour, wired through a laptop running tmux. Color says "energy." Typography says "I've been here all night and I know what every key does."

## What changed from V3

- **Brand renamed** from OneTagger to **DigTrax** (`DIG` foreground + `TRAX` accent, all caps, mono)
- **Typography rebuilt around JetBrains Mono** (single typeface, V2's rule). Dropped Dosis. Dropped Space Grotesk. Display, body, data — all mono.
- **All caps for display + section titles** (V2 convention). Body remains sentence-case for readability.
- **Bracketed labels on actionable controls** (`[ SAVE ]`, `[ APPLY ALL ]`) — V2 rule, kept lighter than V2 (only on primary action affordances, not every button).
- **New Quick Tag layout toggle: Rows vs Cards** (the user-requested change)
- Everything else from V3 stays: color palette, glow halos, vibrancy, radii, spacing, motion, light + dark themes

## Aesthetic
- **Direction:** Maximalist Chaos (color) + Brutalist (typography). A deliberate fusion.
- **Decoration:** color glows + vibrancy panels (V3) + bracketed action labels (V2). No decorative typography.
- **Mood:** energetic and competent. Not friendly. Not unfriendly. Just *credible*.

## Brand mark
Format: `DIG` + `TRAX`
- Both halves uppercase, JetBrains Mono 700, letter-spacing 0.02em
- `DIG` in `--color-fg`
- `TRAX` in `--color-accent` with text-shadow glow `0 0 16px var(--color-accent-glow)` on dark, plain on light

```
DIGTRAX     ← rendered: DIG (white) TRAX (cyan glow)
```

Window title bar: `DIGTRAX — Quick Tag · /Music/Library/2026-Q2`

## Typography
- **Display / titles / labels (uppercase):** **JetBrains Mono** 700, letter-spacing 0.02–0.06em depending on size
- **Body (sentence-case):** **JetBrains Mono** 400 — yes, body is mono. V2 rule.
- **Data:** **JetBrains Mono** 400 with `font-feature-settings: 'tnum'`
- **Fallback:** `"SF Mono", "Berkeley Mono", "Fira Code", monospace`

### Scale
Same as V2 (mono fonts read smaller — body bumps to 14, display larger):
| Token | Size | Usage |
|---|---|---|
| `--text-xs` | 11px | Captions, status |
| `--text-sm` | 13px | Secondary data |
| `--text-base` | 14px | Body |
| `--text-md` | 15px | Emphasized body |
| `--text-lg` | 17px | Section headers (uppercase) |
| `--text-xl` | 22px | Route titles (uppercase) |
| `--text-2xl` | 28px | Page titles (uppercase) |
| `--text-3xl` | 40px | Hero numbers, count callouts |

### Type rules
- All section titles, page titles, sidebar section headers: **UPPERCASE**, mono
- Buttons primary: bracketed text — `[ SAVE CHANGES ]` on the primary action only
- Buttons secondary: plain text, no brackets
- Field labels: uppercase mono, 10–11px, letter-spacing 0.06em (V2's brutalist signature)
- Body text in inspectors, help text: sentence-case, 13–14px (mono but normal case for readability)

## Color (dark, default)
**Identical to V3.** Cool navy base, full-saturation track palette with glow tokens.

| Role | Hex |
|---|---|
| `--color-bg` | `#050811` |
| `--color-bg-elevated` | `#0e131e` |
| `--color-bg-overlay` | `#161c2c` |
| `--color-fg` | `#ffffff` |
| `--color-fg-muted` | `#b8c0d0` |
| `--color-fg-subtle` | `#7a8090` |
| `--color-border` | `rgba(255, 255, 255, 0.08)` |
| `--color-border-strong` | `rgba(255, 255, 255, 0.16)` |
| `--color-accent` | `#00D2BF` |
| `--color-accent-hover` | `#1ee5d2` |
| `--color-accent-glow` | `rgba(0, 210, 191, 0.6)` |

## Color (light)
Identical to V3 with the violet-tinted base.

## Track-color palette
Identical to V3. 8 hues at 100% saturation, each with a `-glow` token.

## Spacing
Identical to V3. 4px base.

## Border radius
Identical to V3. `xs` 6 · `sm` 8 · `md` 12 · `lg` 16 · `xl` 24. Soft radii on mono typography is the unusual fusion — and it works.

## Motion
Identical to V3. Apple-ease, 150 / 240 / 360ms.

## Vibrancy / translucency
Identical to V3. `backdrop-filter: blur(20–30px) saturate(180–200%)` on floating panels with solid fallback for older webkit2gtk.

## Quick Tag — view mode toggle (the new thing)

Two layouts, user-toggled in the toolbar. Persisted in `Settings.quickTag.viewMode`.

### Mode A: ROWS (default)
Single track per line. ~56px tall. Up/down arrow keys move the active row. Designed for fast keyboard tagging through a large library.

```
┌────────────────────────────────────────────────────────────────────────────────┐
│▎  ████  TRACK NAME             ●●●●○  126.0  7A  DREAMY  TECH  ●●●  [⚐]       │
│▎▒  cov  artist                 energy  bpm   key  mood   gnr   cue   stat    │
└────────────────────────────────────────────────────────────────────────────────┘
```

Row composition (left → right):
- 4px color strip (energy color)
- 40px cover thumbnail (with track-color gradient)
- Title (UPPERCASE) + artist stacked, flex
- Energy pips (5 dots, 60px)
- BPM (mono, 60px, tabular-nums)
- Key (mono, 40px)
- Mood chip (one chip; multi-mood shows count "+2")
- Genre (truncate, 100px)
- Hot-cue dots (3 dots in tag colors, 60px)
- Status icon (`[⚐]` if changed, blank if synced)

Active/focused row: 1px accent left border + accent glow halo + title scaled 102%. Selected (multi-select): checkbox visible, accent fill.

### Mode B: CARDS (V3 default, kept as alternate)
The V3 tile grid. 240px tiles. Mini-waveform peek. Glow halos on selected. Best for visual scanning by cover art.

The toggle is a 2-state segmented control in the toolbar:
```
[ROWS] [CARDS]
```
Default: ROWS. The choice persists.

## Keyboard model in Rows mode
- ↑ / ↓ — move focus, auto-scroll
- Space — play/pause focused track
- 1–5 — set energy on focused (or all selected)
- D / F / G / H… — apply mood per binding
- Shift+↓ — extend selection
- ⌘+A — select all visible (after filter)
- Enter — open in Tag Editor for fine edit
- ⌫ — remove from list (no file op)

## Component conventions inheriting from V2
- **Primary action buttons** wrap label in spaces + brackets: `[ SAVE TO 6 FILES ]`
- **Status chips** use `[STATUS]` text instead of color-only badges: `[CHANGED]`, `[SYNCED]`, `[ERROR]`
- **Section headers** in panels use V2's `// section` prefix in muted color, e.g. `// LIBRARY`
- **Field labels** uppercase mono, `--color-fg-subtle`, letter-spacing 0.06em

## Component conventions inheriting from V3
- **Track tiles in card mode** glow in selected tag color
- **Sidebar / inspector / player bar** are translucent vibrancy
- **Hot-cue dots** carry `box-shadow: 0 0 6px currentColor` halos
- **Player play button** is a 44px accent circle with glow
- **Filter chips** active state: accent border + soft background tint

## Brand application notes (post-design rename work)
This is design only. The actual rename touches:
- [Cargo.toml](../../Cargo.toml) workspace member references
- [crates/onetagger/Cargo.toml](../../crates/onetagger/Cargo.toml) — `name`, `description`, `[package.metadata.bundle]` → `name = "DigTrax"`, `identifier = "com.digtrax.app"` (or chosen domain)
- [crates/onetagger-cli/Cargo.toml](../../crates/onetagger-cli/Cargo.toml) — `name = "digtrax-cli"`
- All 11 crate names (`onetagger-*` → `digtrax-*`) — large mechanical change
- [crates/onetagger/src/main.rs](../../crates/onetagger/src/main.rs) and others — `info!("Starting OneTagger")` strings, window titles
- Window title built in `start_webview`
- [README.md](../../README.md), [CHANGELOG.md](../../CHANGELOG.md), [CLAUDE.md](../../CLAUDE.md)
- [assets/](../../assets/) — `onetagger-logo-github.png`, `icon.icns`, `icon.ico`, `installer-icon.ico`, `installer.nsi`
- GitHub Actions artifact names (`OneTagger-mac.zip` → `DigTrax-mac.zip`)
- Optional: GitHub repo rename if forking; default config dir resolved by `directories` (currently `OneTagger` somewhere under `~/Library/Application Support/`) — needs migration logic if existing users carry settings forward

Track this as **Phase 0.5 — DigTrax brand rename** alongside Phase 0 (universal builds). They batch well: one CI rebuild covers both.

## Decisions log
| Date | Decision | Rationale |
|---|---|---|
| 2026-05-04 | Adopt JetBrains Mono everywhere from V2 | User-requested fusion: V3 colors + V2 type rules |
| 2026-05-04 | All-caps for display, sentence-case for body | V2 type rule; readability preserved |
| 2026-05-04 | Brackets on primary actions only | V2 motif kept light — too many brackets read as kitsch |
| 2026-05-04 | Quick Tag Rows mode default | User said "many at a glance" with up/down arrows is the priority workflow |
| 2026-05-04 | Cards mode kept as alternate | User said "I love all the features in the card and don't want to lose them" |
| 2026-05-04 | Brand rename to DigTrax | User-directed. Implies separate Phase 0.5 implementation work |
| 2026-05-04 | Brand mark: DIG fg + TRAX accent, all caps mono | Splits naturally on the "dig for trax" wordplay |
| 2026-05-04 | Keep V3's soft radii (12/16/24) | User asked for V2 fonts, not V2 rectangles |
