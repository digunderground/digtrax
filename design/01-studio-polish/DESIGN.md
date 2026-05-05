# Variant 01 — Studio Apple Polish

## Visual thesis

A 2026 macOS app that happens to be built for DJs. Take Traktor Pro 4's pro-tool DNA — color-coded tracks, prominent waveforms, dense panel layout, monospace for data — and wrap it in Apple's surface treatment: continuous-curvature corners, layered translucent panels with vibrancy, refined typography, light + dark as first-class peers.

**Vibe:** confident, refined, dense-but-breathing. Looks at home next to Logic Pro, Linear, and Notion. Reads as "this is a serious tool that respects your time."

## Aesthetic
- **Direction:** Industrial/Utilitarian core, with intentional decoration
- **Decoration:** subtle surface noise on dark backgrounds (avoids OLED smear), soft accent glows on active elements only
- **Mood:** focus, calm, professionalism. Never overwhelming.

## Typography
- **Display** (logo, route titles, big stats): **Dosis** 600 / 800 — kept from current OneTagger brand. Self-hosted.
- **Body / UI** (everything else): **Geist** 400 / 500 / 600 — Vercel's 2024 font. Geometric humanist. Tabular-nums for BPM / key / year.
- **Data / Mono**: **Geist Mono** 400 / 500 — pairs perfectly. For BPM, durations, file paths, frame names.
- **Fallback**: `-apple-system, "SF Pro Text", system-ui, sans-serif`
- **Loading**: Bunny Fonts CDN in preview; self-hosted (`/client/public/fonts/`) in production

### Scale
| Token | Size | Usage |
|---|---|---|
| `--text-xs` | 11px | Captions, hint text |
| `--text-sm` | 13px | Secondary metadata |
| `--text-base` | 14px | Body, default |
| `--text-md` | 15px | Emphasized body |
| `--text-lg` | 17px | Section headers |
| `--text-xl` | 20px | Route titles |
| `--text-2xl` | 24px | Page titles |
| `--text-3xl` | 32px | Big numbers, splash |

Body at 14px (not 16) is deliberate — DJ tools earn density with tight line-height (1.4) and Geist's clarity.

## Color (dark, default)
| Role | Hex | Notes |
|---|---|---|
| `--color-bg` | `#0c0d0f` | Deep neutral, not pure black (avoids OLED smear) |
| `--color-bg-elevated` | `#16181b` | Cards, panels, sidebar |
| `--color-bg-overlay` | `#1d2024` | Dialogs, popovers, hover states |
| `--color-fg` | `#e8e9ec` | Primary text |
| `--color-fg-muted` | `#9ba0a8` | Secondary text |
| `--color-fg-subtle` | `#6b6f76` | Disabled, hints |
| `--color-border` | `#2a2d33` | 1px default |
| `--color-border-strong` | `#3a3e46` | Focus, selected |
| `--color-accent` | `#00D2BF` | OneTagger teal — kept |
| `--color-accent-hover` | `#1ee5d2` | |
| `--color-accent-glow` | `rgba(0, 210, 191, 0.4)` | Soft halo on focus |
| `--color-success` | `#4ADE80` | |
| `--color-warning` | `#FFB627` | |
| `--color-danger` | `#F87171` | |

## Color (light)
| Role | Hex |
|---|---|
| `--color-bg` | `#fafbfc` |
| `--color-bg-elevated` | `#ffffff` |
| `--color-bg-overlay` | `#ffffff` |
| `--color-fg` | `#0c0d0f` |
| `--color-fg-muted` | `#5b6068` |
| `--color-fg-subtle` | `#8b9098` |
| `--color-border` | `#e4e6ea` |
| `--color-border-strong` | `#cdd0d6` |
| `--color-accent` | `#00B5A4` (saturation reduced for light) |

Light mode is a real redesign, not a CSS invert. Surface elevations re-hierarchize (light → use shadow only, no border tints), accent saturation drops to read on light.

## Track-color palette (energy / mood / custom)
8 saturated hues used as data labels for tags. Same set in both themes.
`#FF6B35` orange · `#FFB627` amber · `#C6F432` lime · `#4ADE80` mint · `#00D2BF` cyan (= accent) · `#38BDF8` sky · `#A78BFA` violet · `#F472B6` rose

## Spacing
4px base, comfortable density.
| Token | px | Usage |
|---|---|---|
| `--space-1` | 4 | Tight, inside chips |
| `--space-2` | 8 | Between related items |
| `--space-3` | 12 | Inside panels |
| `--space-4` | 16 | Between sections |
| `--space-5` | 20 | Generous |
| `--space-6` | 24 | Large gaps |
| `--space-7` | 32 | Page section gaps |
| `--space-8` | 48 | Hero / feature spacing |
| `--space-9` | 64 | Top-level page padding |

## Layout
- **Approach:** hybrid grid — strict 8px discipline for chrome (sidebar, inspector, dialogs), looser content-driven for track grids
- **Sidebar collapsed:** 64px (icons only)
- **Sidebar expanded:** 240px
- **Inspector:** 320px (right pane, optional)
- **Min window:** 1100 × 600 (down from current 1200 × 550 — narrower because sidebar is more efficient than top tabs at small widths)
- **Player bar:** 96px tall (waveform + transport + metadata in one row)

## Border radius — continuous curvature
| Token | px | Usage |
|---|---|---|
| `--radius-xs` | 4 | Inputs, small chips |
| `--radius-sm` | 6 | Buttons |
| `--radius-md` | 10 | Cards, panels |
| `--radius-lg` | 14 | Track tiles, primary surfaces |
| `--radius-xl` | 20 | Dialogs, windows |
| `--radius-full` | 9999 | Avatars, status pills |

Larger than typical DAW. This is the visual signature.

## Motion
- **Easing:** `cubic-bezier(0.2, 0.8, 0.2, 1)` (Apple's standard)
- `--duration-fast` 120ms (hover, button press)
- `--duration-base` 200ms (panels, route change)
- `--duration-slow` 320ms (theme toggle, sidebar collapse)
- Reduced motion respected via `@media (prefers-reduced-motion)`

## Elevation
- **Shadow 1** (cards): `0 1px 2px rgba(0,0,0,0.2), 0 4px 12px rgba(0,0,0,0.15)` + 1px border
- **Shadow 2** (panels): `0 4px 8px rgba(0,0,0,0.25), 0 12px 24px rgba(0,0,0,0.3)` + 1px border
- **Shadow 3** (dialogs): `0 12px 24px rgba(0,0,0,0.4), 0 32px 64px rgba(0,0,0,0.5)` + 1px border

Dark theme shadows are amplified by accent border tints on focus. Light theme is shadow-only.

## Vibrancy / translucency
Floating panels (sidebar overlays, popovers, command palette) use:
```css
background: rgba(22, 24, 27, 0.72);
backdrop-filter: blur(20px) saturate(180%);
-webkit-backdrop-filter: blur(20px) saturate(180%);
```
Falls back to solid `var(--color-bg-overlay)` if `backdrop-filter` is unsupported (older webkit2gtk). Use `@supports` query.

## Decisions log
| Date | Decision | Rationale |
|---|---|---|
| 2026-05-04 | Geist over Inter | Inter is overused; Geist has same clarity + tabular-nums + post-Inter recognizability |
| 2026-05-04 | Body 14px, not 16px | DJ tools are dense; Geist legibility holds at 14 with 1.4 line-height |
| 2026-05-04 | Continuous radii (lg=14, xl=20) | Visual signature; differentiates from DAW conventions |
| 2026-05-04 | Light mode is a first-class peer | Apple-ecosystem citizenship; daytime usability; accessibility |
| 2026-05-04 | Accent kept at #00D2BF | Brand continuity for existing OneTagger users |
