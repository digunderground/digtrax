# Variant 03 — Hot Cue Maximalist

## Visual thesis

Peak-hour club energy in a music-tagging tool. Color is the primary visual element. Track tags glow. Waveforms are loud. The 8-color track palette isn't restrained data labels (V1) or muted backdrop (V2) — it's the front of the brand. Y2K-DJ-flyer meets pro tool. Spotify's "Now Playing" page meets Traktor's hot-cue grid.

**Vibe:** the app is excited that you're here. Performance-mode by default. Reads as "this is for someone who treats their library like a living set, not a spreadsheet."

## Aesthetic
- **Direction:** Maximalist Chaos (controlled) + Industrial undertone
- **Decoration:** expressive — glow halos on active elements, vibrancy backdrops, generous use of accent and track colors
- **Mood:** energy, motion, anticipation. Reads like the moment before the drop.

## Typography
- **Display** (route titles, big stats, callouts): **Space Grotesk** 600 / 700 — wide grotesque, late-2010s techno-magazine vibe. Pairs with the maximalist aesthetic.
- **App-name lockup**: **Dosis** 800 — kept for brand continuity at the logo only
- **Body / UI**: **Geist** 400 / 500 — clean, modern, doesn't fight the display font
- **Data / Mono**: **JetBrains Mono** 400 — for BPM / durations / paths
- **Fallback**: `-apple-system, "SF Pro Text", system-ui, sans-serif`
- **Loading**: Bunny Fonts CDN in preview; self-hosted in production

Three font families is more than V1 — that's part of the maximalist brief.

### Scale
| Token | Size | Usage |
|---|---|---|
| `--text-xs` | 11px | Captions |
| `--text-sm` | 13px | Secondary |
| `--text-base` | 14px | Body |
| `--text-md` | 16px | Emphasized |
| `--text-lg` | 18px | Section headers |
| `--text-xl` | 22px | Route titles (Space Grotesk) |
| `--text-2xl` | 28px | Page titles |
| `--text-3xl` | 40px | Hero numbers — bigger than V1 |
| `--text-4xl` | 56px | Stat callouts (e.g., "238 TRACKS") |

## Color (dark, default)
Slightly cooler base than V1 — a hint of navy underneath the black gives the saturated tags more pop.
| Role | Hex | Notes |
|---|---|---|
| `--color-bg` | `#050811` | Deep navy-black |
| `--color-bg-elevated` | `#0e131e` | Cards, panels |
| `--color-bg-overlay` | `#161c2c` | Dialogs |
| `--color-fg` | `#ffffff` | Pure white — high contrast against navy |
| `--color-fg-muted` | `#b8c0d0` | |
| `--color-fg-subtle` | `#7a8090` | |
| `--color-border` | `rgba(255, 255, 255, 0.08)` | Alpha-based — softens edges |
| `--color-border-strong` | `rgba(255, 255, 255, 0.16)` | Focus |
| `--color-accent` | `#00D2BF` | OneTagger teal — kept |
| `--color-accent-hover` | `#1ee5d2` | |
| `--color-accent-glow` | `rgba(0, 210, 191, 0.6)` | **Heavier than V1** — used for focus halos |
| `--color-success` | `#4ADE80` | |
| `--color-warning` | `#FFB627` | |
| `--color-danger` | `#F87171` | |

## Color (light, secondary)
Light mode exists but is the minority case. Reads more as "lit room" than "default macOS app."
| Role | Hex |
|---|---|
| `--color-bg` | `#f5f3ff` (subtle violet tint, not pure neutral) |
| `--color-bg-elevated` | `#ffffff` |
| `--color-fg` | `#1a1438` (near-black with violet tint) |
| `--color-accent` | `#00B5A4` |

## Track-color palette — front of the brand
Saturation cranked to 100%. Each color gets a `-glow` token for halo effects.
| Color | Hex | Glow (rgba) |
|---|---|---|
| Orange | `#FF6B35` | `rgba(255, 107, 53, 0.5)` |
| Amber | `#FFB627` | `rgba(255, 182, 39, 0.5)` |
| Lime | `#C6F432` | `rgba(198, 244, 50, 0.5)` |
| Mint | `#4ADE80` | `rgba(74, 222, 128, 0.5)` |
| Cyan (= accent) | `#00D2BF` | `rgba(0, 210, 191, 0.5)` |
| Sky | `#38BDF8` | `rgba(56, 189, 248, 0.5)` |
| Violet | `#A78BFA` | `rgba(167, 139, 250, 0.5)` |
| Rose | `#F472B6` | `rgba(244, 114, 182, 0.5)` |

Selected / active track tiles get a glow halo of their tag color (`box-shadow: 0 0 32px var(--track-color-glow)`). This is the visual signature of V3.

## Spacing
4px base, comfortable (matches V1 — the maximalism is in color, not in cramming).
| Token | px |
|---|---|
| `--space-1` | 4 |
| `--space-2` | 8 |
| `--space-3` | 12 |
| `--space-4` | 16 |
| `--space-5` | 20 |
| `--space-6` | 24 |
| `--space-7` | 32 |
| `--space-8` | 48 |
| `--space-9` | 64 |

## Layout
- **Approach:** grid-disciplined for chrome, expressive for content
- **Sidebar collapsed:** 64px
- **Sidebar expanded:** 240px
- **Inspector:** 320px
- **Min window:** 1100 × 600
- **Player bar:** 112px tall (waveform takes more space — performance-tool DNA)

## Border radius — softer than V1
| Token | px |
|---|---|
| `--radius-xs` | 6 |
| `--radius-sm` | 8 |
| `--radius-md` | 12 |
| `--radius-lg` | 16 |
| `--radius-xl` | 24 |
| `--radius-full` | 9999 |

## Motion — expressive
- **Easing:** `cubic-bezier(0.2, 0.8, 0.2, 1)` standard, `cubic-bezier(0.33, 1, 0.68, 1)` for entrance pops
- `--duration-fast` 150ms
- `--duration-base` 240ms
- `--duration-slow` 360ms
- **Glow pulse on hot-cue activation** (200ms scale + glow expand)
- **Track tile hover**: subtle 1.02 scale + glow halo expand
- Reduced-motion: glow halos and scale removed; opacity changes still allowed

## Elevation — glow as elevation
- **Shadow 1** (cards): subtle `0 4px 12px rgba(0, 0, 0, 0.4)` + 1px alpha border
- **Shadow 2** (active surfaces): card shadow + accent glow `0 0 24px var(--color-accent-glow)`
- **Shadow 3** (dialogs): heavy `0 32px 64px rgba(0, 0, 0, 0.6)` + accent glow

## Vibrancy / translucency — heavy
Floating overlays use heavy blur and saturation boost (Spotify-style):
```css
background: rgba(14, 19, 30, 0.6);
backdrop-filter: blur(30px) saturate(200%);
-webkit-backdrop-filter: blur(30px) saturate(200%);
```
Sidebar uses lighter vibrancy (15px blur). Falls back to solid where unsupported.

## Component conventions
- **Track tiles**: large (160px tall), color-coded left strip 6px wide, mini-waveform top-right, big track name in Space Grotesk, BPM/key in Geist Mono
- **Hot-cue indicators**: glowing color dots arranged horizontally on each tile — direct Traktor analogue
- **Buttons primary**: solid accent, soft glow on hover, slightly larger than V1 (44px tall — touch-friendly even though it's a desktop app)
- **Dialogs**: heavy vibrancy, 24px radius, generous padding (`--space-7`)
- **Status chips**: pill-shaped (`--radius-full`), subtle accent glow when "live"

## Decisions log
| Date | Decision | Rationale |
|---|---|---|
| 2026-05-04 | Space Grotesk for display | Wide grotesque pairs with maximalist energy; differentiates from Apple's SF |
| 2026-05-04 | Three font families | Maximalist brief — variation is part of the aesthetic |
| 2026-05-04 | Track colors at 100% saturation with glow tokens | The visual signature; club-energy DNA |
| 2026-05-04 | Cooler base (`#050811`) instead of neutral black | Cool base makes saturated track colors pop more |
| 2026-05-04 | Heavy backdrop-filter blur (30px) | Spotify-style vibrancy; depth as identity |
| 2026-05-04 | Accent glow halos on focused/active elements | Visual rhythm matching hot-cue activations |
| 2026-05-04 | Player bar 112px (taller than V1) | Performance-tool DNA — waveform deserves space |
