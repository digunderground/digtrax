# DigTrax — Final Icon Bundle

The DigTrax app icon, finalized and exported in every format the project needs. Built on the V04 *DigTrax Streamlined* palette: `#050811` navy / `#FFFFFF` foreground / `#00D2BF` cyan accent + glow.

```sh
open design/05-icon-concepts/preview.html
```

## The mark

**Bar cutouts in tag** — the chosen design fuses the C-CUTOUT cutout aesthetic with the C-BARS waveform.

- Centered vinyl with concentric grooves (r = 68 / 104 / 138 / 172) and a glowing cyan accent ring; large white spindle
- Larger rounded luggage-tag (118×78, +17%) overlaps the rings with the right edge slightly breaking the outer circle
- Inside the tag: 11 vertical bar cutouts of varied organic heights, punched through the white body to reveal dark navy beneath, each lit with cyan back-glow and a cyan rim

## Folder layout

```
design/05-icon-concepts/
├── README.md              ← this file
├── preview.html           ← the final review page
├── sources/               ← editable SVG sources
│   ├── digtrax-icon.svg              ← the icon, single source of truth
│   └── digtrax-logo-github.svg       ← horizontal logo + wordmark
└── exports/               ← rendered outputs, ready to ship
    ├── icns/
    │   └── icon.icns                 ← macOS app bundle (10 sizes embedded)
    ├── ico/
    │   ├── icon.ico                  ← Windows app icon (6 sizes)
    │   └── installer-icon.ico        ← Windows installer (same)
    └── png/
        ├── icon-16.png               ← raw PNGs at every size
        ├── icon-32.png
        ├── icon-48.png
        ├── icon-64.png
        ├── icon-128.png
        ├── icon-256.png
        ├── icon-512.png
        ├── icon-1024.png             ← master
        ├── 32x32.png                 ← drop-in replacement for assets/32x32.png
        ├── 128x128.png               ← drop-in replacement for assets/128x128.png
        ├── 128x128@2x.png            ← drop-in replacement for assets/128x128@2x.png
        └── digtrax-logo-github.png   ← replaces assets/onetagger-logo-github.png
```

## How to install in the repo

When the full OneTagger workspace is mounted, copy the exports into `assets/`:

```sh
cd /Users/dig/code/_tools/onetagger

# macOS bundle icon
cp design/05-icon-concepts/exports/icns/icon.icns           assets/icon.icns

# Windows
cp design/05-icon-concepts/exports/ico/icon.ico             assets/icon.ico
cp design/05-icon-concepts/exports/ico/installer-icon.ico   assets/installer-icon.ico

# Tauri / Linux PNGs
cp design/05-icon-concepts/exports/png/32x32.png            assets/32x32.png
cp design/05-icon-concepts/exports/png/128x128.png          assets/128x128.png
cp design/05-icon-concepts/exports/png/128x128@2x.png       assets/128x128@2x.png

# GitHub README banner
cp design/05-icon-concepts/exports/png/digtrax-logo-github.png  assets/onetagger-logo-github.png
# (rename to digtrax-logo-github.png as part of the Phase 0.5 brand rename)
```

The remaining brand-rename work — Cargo manifests, window titles, repo name, config-dir migration — is tracked in [DESIGN.md §Brand application notes](../04-digtrax-streamlined/DESIGN.md) under Phase 0.5.

## Re-rendering from source

If `digtrax-icon.svg` is edited, regenerate everything:

```sh
pip install --break-system-packages cairosvg icnsutil

# PNGs at every size
python3 - <<'PY'
import cairosvg
sizes = [16, 32, 48, 64, 128, 256, 512, 1024]
with open("sources/digtrax-icon.svg", "rb") as f:
    svg = f.read()
for sz in sizes:
    cairosvg.svg2png(bytestring=svg, write_to=f"exports/png/icon-{sz}.png",
                     output_width=sz, output_height=sz)
PY

# .ico bundle (requires ImageMagick)
convert exports/png/icon-{16,32,48,64,128,256}.png exports/ico/icon.ico
cp exports/ico/icon.ico exports/ico/installer-icon.ico

# .icns bundle (requires icnsutil)
python3 - <<'PY'
import icnsutil
img = icnsutil.IcnsFile()
slots = {'ic04':16,'ic11':32,'ic05':32,'ic12':64,'ic07':128,'ic13':256,
         'ic08':256,'ic14':512,'ic09':512,'ic10':1024}
for code, sz in slots.items():
    img.add_media(code, file=f"exports/png/icon-{sz}.png")
img.write("exports/icns/icon.icns")
PY
```

## Concept exploration archive

Earlier rounds of icon exploration (vinyl + tonearm, D-monogram, crate, EQ-bar tag, smooth wave, sharp wave, dual envelope, etc.) lived under `icons/_archive/` while we were iterating. They were cleared from the workspace when the design was locked. If you want to revisit them, check git history.
