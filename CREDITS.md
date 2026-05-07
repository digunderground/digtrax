# DigTrax — Third-Party Attribution

DigTrax is © 2024-2026 Adam Tout, distributed under the GNU General
Public License version 3 (see [`LICENSE`](LICENSE)). DigTrax was
originally forked from [OneTagger](https://github.com/Marekkon5/onetagger)
by Marekkon5 (also GPL-3) and has been substantially redesigned and
extended for DJ workflows.

This file documents the third-party software DigTrax incorporates,
along with their licenses and copyright holders. The full license text
for each component is included in the project under the path noted
beside it.

---

## QM-DSP — beat detection for the DJ Mode dual-deck mixer

DigTrax's DJ Mode (introduced in v1.8.0) uses the
[QM-DSP](https://github.com/c4dm/qm-dsp) library — the same beat
detection engine that ships in [Mixxx](https://github.com/mixxxdj/mixxx).
The relevant QM-DSP source files are vendored verbatim under
[`crates/digtrax-deck/vendor/qm-dsp/`](crates/digtrax-deck/vendor/qm-dsp/)
so the build is self-contained.

QM-DSP is a C++ library of digital signal processing functions for
music informatics, developed by the [Centre for Digital
Music](http://c4dm.eecs.qmul.ac.uk) at Queen Mary, University of
London.

### Files vendored

- `dsp/onsets/DetectionFunction.{cpp,h}` — Complex Spectral Difference
  onset detection
- `dsp/tempotracking/TempoTrackV2.{cpp,h}` — Davies-Plumbley tempo
  tracker with Viterbi decoding + Ellis dynamic-programming beat
  tracker
- `dsp/phasevocoder/PhaseVocoder.{cpp,h}` — phase vocoder for the
  detection function's phase predictions
- `dsp/transforms/FFT.{cpp,h}` — FFT wrapper around kissfft
- `maths/MathUtilities.{cpp,h}` — `princarg`, adaptive thresholding,
  numerical helpers
- `base/Window.h` — Hanning / Hamming / Blackman window template
- `ext/kissfft/` — Mark Borgerding's
  [kissfft](https://github.com/mborgerding/kissfft), used internally
  by the FFT wrapper

### Copyright

> Copyright © 2005-2018 Centre for Digital Music, Queen Mary,
> University of London.
> 2008-2009 portions copyright Matthew Davies and QMUL
> (`TempoTrackV2.cpp`).
> 2005-2006 portions copyright Christian Landone
> (`DetectionFunction.cpp`).

### License

QM-DSP is licensed under the **GNU General Public License version 2 or
(at your option) any later version**. The full text is at
[`crates/digtrax-deck/vendor/qm-dsp/COPYING`](crates/digtrax-deck/vendor/qm-dsp/COPYING).

DigTrax statically links QM-DSP, so under the terms of the GPL-2-or-
later DigTrax is distributed as **GPL-3** as a whole.

### Attribution to Mixxx

The code paths and constants used by DigTrax's DJ Mode (step size
`sample_rate × 0.01161`, window size `nextPow2(sample_rate / 50)`,
`DF_COMPLEXSD`, centered windowing, skip-first-2 trim, beat sample
offset of `step / 2`) are taken directly from
[Mixxx](https://github.com/mixxxdj/mixxx) v2.7-alpha's
`src/analyzer/plugins/analyzerqueenmarybeats.cpp`. The thin C wrapper
at [`crates/digtrax-deck/vendor/qm_shim.cc`](crates/digtrax-deck/vendor/qm_shim.cc)
mirrors that pipeline. Mixxx is licensed under the GNU General Public
License version 2 or (at your option) any later version.

> Mixxx © 2001-2026 The Mixxx Development Team.

---

## kissfft

Bundled with QM-DSP under `vendor/qm-dsp/ext/kissfft/`. Copyright ©
2003-2010 Mark Borgerding. Licensed under the BSD 3-Clause License.

---

## Other dependencies

The full transitive Rust crate graph and its licenses can be inspected
via:

```sh
cargo tree -p digtrax-deck --format "{p} {l}"
```

Notable deps and their licenses:

| Crate | Use | License |
|-------|-----|---------|
| `cpal` | Cross-platform audio I/O | MIT OR Apache-2.0 |
| `rubato` | Sample-rate conversion (placeholder for v2 time-stretch) | MIT |
| `realfft` | Used in places we haven't yet ported to QM-DSP | MIT/Apache-2.0 |
| `symphonia` | Audio file decoding | MPL-2.0 |
| `crossbeam-channel` | Lock-free SPSC for non-RT control commands | MIT/Apache-2.0 |
| `cc` (build-dep) | C++ compilation of vendored QM-DSP | MIT/Apache-2.0 |

Front-end dependencies (Vue 3, Quasar, Vite) and back-end dependencies
shared with the rest of DigTrax are unchanged from prior releases —
their attributions live in their respective `LICENSE` files.
