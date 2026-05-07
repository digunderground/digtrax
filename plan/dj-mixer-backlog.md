# DJ Mixer — v2 Backlog

Features deferred from the initial DJ Mixer ship (`feature/dj-mixer`).
Not yet committed to v2; this is a parking lot, not a roadmap.

## v1 ship state

What landed in `feature/dj-mixer`:
- New `crates/digtrax-deck/` (cpal-based, no Qt, MIT/Apache-2.0)
- 2 decks with drag-drop from Quick Tag
- Symphonia decode → in-memory f32 stereo
- Per-deck rate slider (pitch-coupled, vinyl-style)
- Equal-power crossfader, master gain
- Kick-focused beat tracker (30–150 Hz envelope + ACF + Gaussian
  prior at 128 BPM, 7/7 synthetic-click tests pass at ±0.5 BPM)
- Scrolling spectrum waveform (RGB low/mid/high) with beat-grid
  markers (yellow every 4, cyan every 16) drawn from the actual
  detected beat sequence
- Closed-loop sync engine (PI controller in audio callback,
  Mixxx-style; gain 0.7, ±2% rate cap, train-wreck threshold 20%
  of a beat)
- 3-band EQ (low-shelf 250 Hz, mid peaking 1.5 kHz, high-shelf
  5 kHz; cookbook biquads, kill at 0.0)
- DJ filter knob (single sweep through LPF → bypass → HPF, 100 Hz
  to 12 kHz log range)
- Beat jump (-4, -1, +1, +4 beats, quantized to bracket beat)
- DJ Mode toggle in PlayerBar (preview path untouched when off)

## Audio
- **Hot cues** (8 per deck, with color-coded states like Mixxx). v1 ships zero
  cue points; the engine has no cue infrastructure at all yet.
- **Loops** (1/2/4/8-beat in/out + active toggle). Needs a loop buffer in the
  audio thread and a beat-aware loop renderer on the waveform.
- **Headphone cue routing**. Requires picking a second cpal output device, a
  per-deck PFL toggle, master/cue blend knob (HEAD/MIX), and SPLIT-cue mode
  (mono master → L, mono cue → R). Significant cpal multi-device work.
- **Master balance / booth out / split-cue** in the master section (v1 only
  has master gain).
- **FX1 / FX2 send chain** — per-deck send amount + master FX rack with at
  least filter, echo, reverb. Mixxx ships these built-in; their `EffectChain`
  + `EffectsBackend` are all Qt-coupled, so we'd port the algorithms (most
  are biquad/delay-line) and build our own chain manager.
- **Key shifter** — pitch shift independent of tempo. RubberBand does this
  natively; with rubato we'd need a separate phase vocoder pass.
- **Recording / broadcasting** — bounce-to-disk of the master output, or
  Icecast streaming.
- **MIDI controller mapping** — XML mapping schema like Mixxx so users can
  bind hardware faders/knobs to DigTrax controls.

## DSP refinements
- **Time-stretch fallback to SoundTouch FFI.** Decision deferred to end of
  Phase 2. If rubato's transient smear at ±20% rate is unacceptable, swap in
  SoundTouch via cc/cmake. LGPL via dynamic link is acceptable in this
  private repo.
- **Beat tracker fallback to QM-DSP CSD port.** The current kick-tracker is
  light and works on synthetic clicks; `research/mixxx/lib/qm-dsp/` is the
  Mixxx-grade option if real-world EDM tracks fail Phase 2 verification.
- **Quantize-on-seek**. When grid is known, snap user-initiated seeks to
  the nearest beat (Mixxx `quantize` flag).

## Metering (deferred from Phase 5)
- **Per-deck VU meter** (peak + RMS) — visible beside each deck.
  Audio thread already touches the buffer; cheap to compute. Push
  alongside djPosition at 33 Hz.
- **Master VU meter** in the center column (post-crossfader).

## UX
- **Per-deck rate slider with center detent + nudge buttons** (we ship the
  knob value; Mixxx's tempo fader UX is more nuanced).
- **Visual sync indicator** — a flashing border or pulse animation when
  the deck is locked to the leader.
- **Drag-to-deck preview**. Hover over a deck slot during drag = audition
  the source on cue without committing the load.
- **Per-deck sub-bar phrase markers**. v1 ships every-16-beat phrase
  markers; finer phrase analysis (intro/verse/breakdown/drop) is a v2
  candidate.

## License notes
- v1 stays MIT/Apache-2.0 across all dependencies (cpal, rubato, realfft,
  symphonia, crossbeam, anyhow).
- SoundTouch (LGPL) is acceptable as a v1.x fallback if rubato disappoints.
- RubberBand (GPL-2) would force the whole DigTrax binary to GPL-2; only
  consider after consultation with the user on the licensing implications.
