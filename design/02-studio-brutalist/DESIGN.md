# Variant 02 — Studio Brutalist

## Visual thesis

A power-tool. No polish, no decoration, no apologies. Function exposed. The design language of UNIX debuggers and Bloomberg terminals applied to DJ tagging. Mono font everywhere. Hard rectangles. One accent color (electric phosphor green). Dark only. The visual equivalent of a tool that says "I am here to do work, not to charm you."

**Vibe:** terminal-grade, hacker-credible, anti-marketing. Like Kagi, Cursor, or the better parts of `htop` brought into a DJ tool. Will alienate some users; the ones who stay will love it forever.

## Aesthetic
- **Direction:** Brutalist + Industrial
- **Decoration:** zero. No gradients, no shadows, no rounded corners, no glow, no transitions beyond instant snap.
- **Mood:** focus, function, no-BS. Reads as "I respect your time so much I refuse to entertain you."

## Typography
- **Display:** **JetBrains Mono** 700 — same family as body, bumped weight
- **Body / UI:** **JetBrains Mono** 400 / 500 — single typeface across the entire app
- **Data / Mono:** **JetBrains Mono** 400 — already mono everywhere
- **Fallback:** `"SF Mono", "Berkeley Mono", "Fira Code", monospace`
- **Loading:** Bunny Fonts CDN in preview; self-hosted in production

The **Dosis** brand font is dropped in this variant. It's anti-thematic.

### Scale
Slightly larger than V1 since mono fonts read smaller at the same px. Body bumps to 15.
| Token | Size | Usage |
|---|---|---|
| `--text-xs` | 12px | Captions, status |
| `--text-sm` | 13px | Secondary metadata |
| `--text-base` | 15px | Body, default |
| `--text-md` | 16px | Emphasized body |
| `--text-lg` | 18px | Section headers |
| `--text-xl` | 21px | Route titles |
| `--text-2xl` | 26px | Page titles |
| `--text-3xl` | 34px | Big numbers |

## Color (dark only)
| Role | Hex | Notes |
|---|---|---|
| `--color-bg` | `#0a0a0a` | Near black — not pure, but very close |
| `--color-bg-elevated` | `#141414` | Panels |
| `--color-bg-overlay` | `#1c1c1c` | Hover, dialogs |
| `--color-fg` | `#f4f4f4` | Primary text |
| `--color-fg-muted` | `#888888` | Secondary text |
| `--color-fg-subtle` | `#5a5a5a` | Disabled, hints |
| `--color-border` | `#2a2a2a` | 1px default |
| `--color-border-strong` | `#444444` | Focus, selected |
| `--color-accent` | `#00ff88` | Phosphor green — single accent |
| `--color-accent-hover` | `#3dffa3` | |
| `--color-success` | `#00ff88` | Same as accent — semantic merge |
| `--color-warning` | `#ffcc00` | Yellow, no shades |
| `--color-danger` | `#ff3344` | Red, no shades |

**No light mode.** Light mode would betray the aesthetic. If a user demands it, fall back to a high-contrast inverted palette at the OS level — but it's unsupported.

## Track-color palette
Same 8-hue set as V1 but **desaturated to ~70%** so they read as labels, not decoration. They're functional, not expressive.
`#cc6b35` · `#ccaa27` · `#a4cc32` · `#3dcc6e` · `#1d9b91` · `#3893b8` · `#7d70b0` · `#bb6090`

## Spacing
**Tighter base unit (3px instead of 4)** — feels denser, more terminal-like.
| Token | px |
|---|---|
| `--space-1` | 3 |
| `--space-2` | 6 |
| `--space-3` | 9 |
| `--space-4` | 12 |
| `--space-5` | 18 |
| `--space-6` | 24 |
| `--space-7` | 36 |
| `--space-8` | 48 |

## Layout
- **Approach:** strict grid-disciplined. Visible 1px borders between every panel. No floating shadows.
- **Sidebar collapsed:** 56px
- **Sidebar expanded:** 220px
- **Inspector:** 300px
- **Min window:** 1024 × 600
- **Player bar:** 80px

## Border radius
- `--radius-xs` 0 (zero — true rectangles)
- `--radius-sm` 0
- `--radius-md` 2 (very subtle, only on focus rings)
- `--radius-lg` 2
- `--radius-xl` 0
- `--radius-full` 0 (no pills — status uses `[STATUS]` text in brackets instead)

Hard rectangles communicate the brutalist thesis instantly.

## Motion
- **Standard:** instant (0ms) for nearly everything
- **Allowed:** 60ms snap for menu open / panel reveal — just enough to confirm the action
- **Forbidden:** fade, slide, scale, blur transitions
- Reduced-motion users see nothing change (since baseline is already minimal)

## Elevation
**No shadows.** Surfaces are differentiated by 1px `--color-border` only. Hover state inverts: `--color-bg-elevated` becomes `--color-bg-overlay` with no other change.

## Vibrancy / translucency
**None.** All surfaces are solid. `backdrop-filter` is not used.

## Component conventions
- Buttons: `[ EXECUTE ]` style — bracketed text labels, often uppercase, mono by default
- Status: `[OK]` `[WAIT]` `[FAIL]` text labels in bracket-pill style instead of color-only badges
- Dropdowns: full-width borders, no rounded edges, options listed in caps
- Inputs: 1px border bottom only on focused, no full-border boxes
- Toggles: `[X]` / `[ ]` checkboxes; no iOS-style sliders
- Progress: `[#####.....] 50%` ASCII style, with the option of a thin bar fallback

## Decisions log
| Date | Decision | Rationale |
|---|---|---|
| 2026-05-04 | Single mono typeface across whole app | Brutalist thesis: function over variation |
| 2026-05-04 | Dosis dropped | Anti-thematic — Dosis is friendly; this variant is not |
| 2026-05-04 | No light mode | Light mode would betray the terminal aesthetic |
| 2026-05-04 | Phosphor green accent | Iconic terminal color; immediate signal of "this is a power tool" |
| 2026-05-04 | Zero motion | Instant feels honest; transitions feel decorative |
| 2026-05-04 | 3px spacing base | Denser than V1; matches mono-font column rhythm |
