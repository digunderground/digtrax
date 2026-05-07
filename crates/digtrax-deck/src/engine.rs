//! Per-deck playback engine.
//!
//! Two halves:
//!
//! - [`DeckEngine`] lives entirely on the audio thread. Its `process()`
//!   method is called by the cpal callback every buffer (~10 ms). It
//!   reads from a pre-decoded sample buffer, applies linear-interpolated
//!   sample-rate conversion if the source rate differs from the cpal
//!   output rate, scales by the user volume, and writes to the deck's
//!   per-buffer scratch space. Phase 2 inserts rubato (true SRC + time
//!   stretch) here; Phase 4 inserts the EQ + filter chain.
//!
//! - [`DeckHandle`] is a `Send + Clone` controller. The WS layer holds
//!   one and uses it from any thread to load files, play/pause/seek,
//!   adjust volume. Communication is **lock-free**: continuous values
//!   (volume, position) go through atomic `f32`/`u64`s; one-shot
//!   commands (load, seek) go through a `crossbeam_channel` that the
//!   audio thread `try_recv()`s on every buffer (never blocks).
//!
//! Atomic-f32 trick: Rust has no `AtomicF32`, so we store
//! `f32::to_bits() -> u32` in an `AtomicU32` and `f32::from_bits()` on
//! the read side. Cheap and well-defined.

use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

use anyhow::{anyhow, Error};
use crossbeam_channel::{Receiver, Sender, TrySendError};

use crate::decode::DecodedAudio;
use crate::eq::EqChain;
use crate::filter::FilterChain;

/// Snapshot of a deck's user-visible state — what we push back to the
/// frontend over WS.
#[derive(Debug, Clone)]
pub struct DeckSnapshot {
    /// Current playback position in milliseconds (file-time).
    pub position_ms: u64,
    /// Total track length in milliseconds, 0 if no track loaded.
    pub duration_ms: u64,
    /// Whether the deck is producing audio right now (`Play` active AND
    /// haven't run off the end of the source).
    pub playing: bool,
    /// User volume in [0.0, 1.0].
    pub volume: f32,
    /// Effective rate multiplier (1.0 = native). Pushed to the front-end
    /// so the BPM badge can show effective playback BPM (file BPM × rate).
    pub rate: f32,
    /// True if a track is loaded.
    pub loaded: bool,
}

/// 3-band EQ band selector for `DeckHandle::set_eq`.
#[derive(Debug, Clone, Copy)]
pub enum EqBand { Low, Mid, High }

/// Non-RT command sent from a `DeckHandle` to the audio thread.
pub(crate) enum DeckCommand {
    Load(DecodedAudio),
    Unload,
    Play,
    Pause,
    /// Seek to the given source-frame index.
    Seek(u64),
    /// Replace the beat sequence (in source frames) and BPM. Sent after
    /// the analyzer finishes so the audio thread has the data it needs
    /// for sync-engine beat-distance math.
    SetBeats { beats_frames: Vec<u64>, bpm: f32 },
    /// Quantized seek: jump `n` beats from the current bracket beat.
    /// Negative = backward. Clamped to the beats array bounds. No-op
    /// if no beats are loaded yet.
    BeatJump(i32),
}

/// Audio-thread state for one deck. Owned by the cpal callback closure.
pub(crate) struct DeckEngine {
    /// Currently loaded track, if any. `None` = silence.
    audio: Option<DecodedAudio>,
    /// Read position in **source frames** (not output frames). Floating
    /// point because at non-1.0 SRC ratios we step by fractional frames.
    position_frames: f64,
    /// cpal output sample rate in Hz. Per-deck SRC ratio is derived
    /// per-buffer from `audio.sample_rate / output_rate`.
    output_rate: u32,
    /// Number of output channels (cpal callback's frame width).
    output_channels: u16,
    /// Inbound command queue. The audio thread drains this on every
    /// buffer with `try_recv()`.
    cmd_rx: Receiver<DeckCommand>,
    /// Detected beat positions in **source frames**, monotonically
    /// increasing. Used by `beat_distance()` for the sync engine.
    /// Empty until the analyzer finishes (~2-3s after Load).
    beats_frames: Vec<u64>,
    /// File-native BPM from the analyzer. 0 = analysis not done.
    /// Used by the sync engine to compute the base rate ratio.
    file_bpm: f32,
    /// 3-band EQ chain (state + cached coefficients). Recomputed only
    /// when a knob actually moves; per-frame cost is one biquad-tick
    /// per band per channel.
    eq: EqChain,
    /// DJ filter chain (single-knob LPF↔bypass↔HPF sweep).
    filter: FilterChain,
    /// Read-side handle — atomics this engine writes for outside readers.
    handle: DeckHandle,
}

/// Send-able controller for a deck. The WS layer holds these.
#[derive(Clone)]
pub struct DeckHandle {
    cmd_tx: Sender<DeckCommand>,
    /// f32 bits — user volume in [0.0, 1.0].
    volume: Arc<AtomicU32>,
    /// f32 bits — user rate multiplier. 1.0 = native speed; 1.5 = 1.5×
    /// faster (and 1.5× higher pitch — pitch-coupled until Phase 2c
    /// adds a phase vocoder or SoundTouch FFI). Clamped to [0.5, 2.0]
    /// in `set_rate`.
    rate: Arc<AtomicU32>,
    /// True while the deck is producing audio. Audio thread writes;
    /// readers (WS push) read.
    playing: Arc<AtomicBool>,
    /// Source frame index of the playhead. Audio thread writes once per
    /// buffer; readers convert to ms via `source_rate`.
    position_frames: Arc<AtomicU64>,
    /// Total source frames (set by Load). 0 = no track loaded.
    duration_frames: Arc<AtomicU64>,
    /// Source sample rate (set by Load). 0 = no track loaded.
    source_rate: Arc<AtomicU32>,
    /// 3-band EQ knobs, each f32 bits. Range [0.0, 2.0]. 1.0 = unity,
    /// 0.0 = full kill, 2.0 = +6 dB. Frontend's HML knob trio writes
    /// these via `set_eq()`.
    eq_low: Arc<AtomicU32>,
    eq_mid: Arc<AtomicU32>,
    eq_high: Arc<AtomicU32>,
    /// DJ-style filter knob, f32 bits, range [-1.0, 1.0].
    /// 0 = bypass, negative = LPF sweep, positive = HPF sweep.
    filter_knob: Arc<AtomicU32>,
}

impl DeckHandle {
    /// Set the deck volume. `value` is clamped to [0.0, 1.0].
    pub fn set_volume(&self, value: f32) {
        let v = value.clamp(0.0, 1.0);
        self.volume.store(v.to_bits(), Ordering::Release);
    }

    /// Set the playback rate multiplier. 1.0 = native, 1.5 = 1.5× speed
    /// (and 1.5× higher pitch — pitch-coupling is acknowledged tech
    /// debt; Phase 2c swaps in pitch-preserving stretch if needed).
    /// Clamped to [0.5, 2.0]. Phase 3's sync controller uses this same
    /// atomic to nudge the slave deck.
    pub fn set_rate(&self, value: f32) {
        let v = value.clamp(0.5, 2.0);
        self.rate.store(v.to_bits(), Ordering::Release);
    }

    /// Read the current rate multiplier — used by the WS push so the
    /// frontend's BPM badge can show the effective playback BPM
    /// (bpm × rate).
    pub fn rate(&self) -> f32 {
        f32::from_bits(self.rate.load(Ordering::Acquire))
    }

    /// Begin playback. Idempotent — calling Play on an already-playing
    /// deck does nothing. Has no effect on an empty deck.
    pub fn play(&self) {
        let _ = self.cmd_tx.try_send(DeckCommand::Play);
    }

    /// Pause. Idempotent.
    pub fn pause(&self) {
        let _ = self.cmd_tx.try_send(DeckCommand::Pause);
    }

    /// Pause + reset position to start.
    pub fn stop(&self) {
        let _ = self.cmd_tx.try_send(DeckCommand::Pause);
        let _ = self.cmd_tx.try_send(DeckCommand::Seek(0));
    }

    /// Seek to a position in milliseconds (file-time). Lossy round-trip
    /// through frames; sub-millisecond accuracy isn't promised.
    pub fn seek_ms(&self, pos_ms: u64) {
        let sr = self.source_rate.load(Ordering::Acquire);
        if sr == 0 { return; }
        let frame = (pos_ms as f64 * sr as f64 / 1000.0).round() as u64;
        let _ = self.cmd_tx.try_send(DeckCommand::Seek(frame));
    }

    /// Replace the loaded track. Returns `Err` if the audio thread's
    /// command queue is full (32 deep — practically unhittable).
    pub fn load(&self, audio: DecodedAudio) -> Result<(), Error> {
        self.cmd_tx
            .try_send(DeckCommand::Load(audio))
            .map_err(|e| match e {
                TrySendError::Full(_) => anyhow!("deck command queue full"),
                TrySendError::Disconnected(_) => anyhow!("audio thread disconnected"),
            })
    }

    /// Drop the loaded track and stop output.
    pub fn unload(&self) {
        let _ = self.cmd_tx.try_send(DeckCommand::Unload);
    }

    /// Set one EQ band. `band` selects low/mid/high; `value` ∈ [0, 2]
    /// where 1 = unity, 0 = kill, 2 = +6 dB.
    pub fn set_eq(&self, band: EqBand, value: f32) {
        let v = value.clamp(0.0, 2.0);
        let target = match band {
            EqBand::Low => &self.eq_low,
            EqBand::Mid => &self.eq_mid,
            EqBand::High => &self.eq_high,
        };
        target.store(v.to_bits(), Ordering::Release);
    }

    /// Set the filter knob. `value` ∈ [-1, 1]. 0 = bypass.
    pub fn set_filter(&self, value: f32) {
        let v = value.clamp(-1.0, 1.0);
        self.filter_knob.store(v.to_bits(), Ordering::Release);
    }

    /// Jump `n` beats from the current bracket beat. Negative = back.
    /// No-op if beats haven't been analyzed yet.
    pub fn beat_jump(&self, n: i32) {
        let _ = self.cmd_tx.try_send(DeckCommand::BeatJump(n));
    }

    /// Provide the audio thread with the analyzer's beat sequence.
    /// `beats_ms` are in milliseconds (file-time); we convert to source
    /// frames before sending so the audio thread can binary-search
    /// against `position_frames` directly.
    pub fn set_beats(&self, beats_ms: &[u64], bpm: f32) {
        let sr = self.source_rate.load(std::sync::atomic::Ordering::Acquire);
        if sr == 0 { return; }
        let beats_frames: Vec<u64> = beats_ms.iter()
            .map(|&ms| (ms as f64 * sr as f64 / 1000.0).round() as u64)
            .collect();
        let _ = self.cmd_tx.try_send(DeckCommand::SetBeats { beats_frames, bpm });
    }

    /// Snapshot the deck's current state for the WS push layer.
    pub fn snapshot(&self) -> DeckSnapshot {
        let sr = self.source_rate.load(Ordering::Acquire);
        let pos_frames = self.position_frames.load(Ordering::Acquire);
        let dur_frames = self.duration_frames.load(Ordering::Acquire);
        let to_ms = |frames: u64| -> u64 {
            if sr == 0 { 0 } else { (frames as f64 * 1000.0 / sr as f64).round() as u64 }
        };
        DeckSnapshot {
            position_ms: to_ms(pos_frames),
            duration_ms: to_ms(dur_frames),
            playing: self.playing.load(Ordering::Acquire),
            volume: f32::from_bits(self.volume.load(Ordering::Acquire)),
            rate: f32::from_bits(self.rate.load(Ordering::Acquire)),
            loaded: dur_frames > 0,
        }
    }
}

impl DeckEngine {
    /// Construct a new engine + handle pair. The engine is moved into
    /// the cpal callback closure; the handle is given to the WS layer.
    pub(crate) fn new(output_rate: u32, output_channels: u16) -> (DeckEngine, DeckHandle) {
        // Capacity 32 is enough for any plausible burst (Load can fire
        // alongside Play, Seek, Volume changes; we never queue more than
        // a handful per UI action).
        let (cmd_tx, cmd_rx) = crossbeam_channel::bounded(32);
        let volume = Arc::new(AtomicU32::new(0.7_f32.to_bits()));
        let rate = Arc::new(AtomicU32::new(1.0_f32.to_bits()));
        let playing = Arc::new(AtomicBool::new(false));
        let position_frames = Arc::new(AtomicU64::new(0));
        let duration_frames = Arc::new(AtomicU64::new(0));
        let source_rate = Arc::new(AtomicU32::new(0));
        let eq_low = Arc::new(AtomicU32::new(1.0_f32.to_bits()));
        let eq_mid = Arc::new(AtomicU32::new(1.0_f32.to_bits()));
        let eq_high = Arc::new(AtomicU32::new(1.0_f32.to_bits()));
        let filter_knob = Arc::new(AtomicU32::new(0.0_f32.to_bits()));
        let handle = DeckHandle {
            cmd_tx,
            volume,
            rate,
            playing,
            position_frames,
            duration_frames,
            source_rate,
            eq_low,
            eq_mid,
            eq_high,
            filter_knob,
        };
        let engine = DeckEngine {
            audio: None,
            position_frames: 0.0,
            output_rate,
            output_channels,
            cmd_rx,
            beats_frames: Vec::new(),
            file_bpm: 0.0,
            eq: EqChain::new(output_rate),
            filter: FilterChain::new(output_rate),
            handle: handle.clone(),
        };
        (engine, handle)
    }

    /// File-native BPM from the analyzer. The sync engine reads this
    /// during the audio callback to compute `base_rate = leader.bpm /
    /// follower.bpm`. 0 = analysis not done.
    pub(crate) fn file_bpm(&self) -> f32 { self.file_bpm }

    /// Audio-thread side: whether playback is engaged (last value
    /// stored to `handle.playing`). Used by the sync engine to skip
    /// corrections when either deck is paused — otherwise the PI loop
    /// can snap-seek the follower against a frozen leader phase, which
    /// makes the visible waveform jump on every audio buffer.
    pub(crate) fn is_playing(&self) -> bool {
        self.handle.playing.load(Ordering::Acquire)
    }

    /// Current playhead in source frames (audio-thread side, before
    /// the per-buffer atomic store).
    pub(crate) fn position_frames(&self) -> f64 { self.position_frames }

    /// Read-only view of the detected beat positions (in source frames).
    /// Used by the sync engine's snap-seek to align the follower's
    /// nearest beat with the leader's phase on sync engage.
    pub(crate) fn beats_frames(&self) -> &[u64] { &self.beats_frames }

    /// Direct write of the playhead. Called by the sync engine in the
    /// audio callback when it engages sync — does a one-shot snap to
    /// the leader-aligned beat so the PI controller doesn't have to
    /// drag the follower across half a beat at the rate cap. Bypasses
    /// the command channel because we're already on the audio thread.
    pub(crate) fn write_position_frames(&mut self, f: f64) {
        let clamped = if let Some(audio) = &self.audio {
            f.clamp(0.0, audio.frames.saturating_sub(1) as f64)
        } else {
            0.0
        };
        self.position_frames = clamped;
        self.handle.position_frames.store(clamped as u64, std::sync::atomic::Ordering::Release);
    }

    /// Beat distance: phase fraction in `[0.0, 1.0)` between the
    /// previous beat and the next beat. Returns `None` if the deck
    /// has no beats yet (analysis pending), no track loaded, or the
    /// playhead sits before beat 0 / past the last beat.
    ///
    /// Used by the sync engine to compute the phase error between
    /// leader and follower in the audio callback. Sample-accurate
    /// because both values come from the same callback iteration.
    pub(crate) fn beat_distance(&self) -> Option<f64> {
        if self.beats_frames.len() < 2 { return None; }
        let pos = self.position_frames;
        let beats = &self.beats_frames;
        // Binary-search the first beat strictly greater than `pos`.
        // i.e. the upper bracket. The lower bracket is one before it.
        let mut lo = 0usize;
        let mut hi = beats.len();
        while lo < hi {
            let mid = (lo + hi) / 2;
            if (beats[mid] as f64) <= pos { lo = mid + 1; } else { hi = mid; }
        }
        // lo is the index of the first beat > pos. We need a bracket
        // with one beat behind and one ahead.
        if lo == 0 || lo >= beats.len() { return None; }
        let prev = beats[lo - 1] as f64;
        let next = beats[lo] as f64;
        if next <= prev { return None; }
        Some(((pos - prev) / (next - prev)).clamp(0.0, 1.0 - f64::EPSILON))
    }

    /// Direct rate write — used by `SyncEngine` running in the audio
    /// callback. Bypasses the public `DeckHandle::set_rate` clamp
    /// because sync corrections should already be within ±2% of the
    /// base rate (well inside [0.5, 2.0]).
    pub(crate) fn write_rate(&self, rate: f32) {
        self.handle.rate.store(rate.to_bits(), std::sync::atomic::Ordering::Release);
    }

    /// Drain pending commands from the WS layer. Called once per audio
    /// buffer at the top of `process`.
    fn drain_commands(&mut self) {
        while let Ok(cmd) = self.cmd_rx.try_recv() {
            match cmd {
                DeckCommand::Load(audio) => {
                    self.handle.duration_frames.store(audio.frames, Ordering::Release);
                    self.handle.source_rate.store(audio.sample_rate, Ordering::Release);
                    self.position_frames = 0.0;
                    self.handle.position_frames.store(0, Ordering::Release);
                    self.handle.playing.store(false, Ordering::Release);
                    self.audio = Some(audio);
                }
                DeckCommand::Unload => {
                    self.audio = None;
                    self.position_frames = 0.0;
                    self.handle.duration_frames.store(0, Ordering::Release);
                    self.handle.source_rate.store(0, Ordering::Release);
                    self.handle.position_frames.store(0, Ordering::Release);
                    self.handle.playing.store(false, Ordering::Release);
                }
                DeckCommand::Play => {
                    if self.audio.is_some() {
                        self.handle.playing.store(true, Ordering::Release);
                    }
                }
                DeckCommand::Pause => {
                    self.handle.playing.store(false, Ordering::Release);
                }
                DeckCommand::Seek(frame) => {
                    if let Some(a) = &self.audio {
                        let f = frame.min(a.frames.saturating_sub(1));
                        self.position_frames = f as f64;
                        self.handle.position_frames.store(f, Ordering::Release);
                    }
                }
                DeckCommand::SetBeats { beats_frames, bpm } => {
                    self.beats_frames = beats_frames;
                    self.file_bpm = bpm;
                }
                DeckCommand::BeatJump(n) => {
                    if self.beats_frames.is_empty() { continue; }
                    let pos = self.position_frames as u64;
                    // Bracket beat: the largest beat ≤ current position.
                    let current_idx = self.beats_frames
                        .iter()
                        .rposition(|&b| b <= pos)
                        .unwrap_or(0);
                    let last = self.beats_frames.len() as i32 - 1;
                    let target_idx = (current_idx as i32 + n).clamp(0, last) as usize;
                    let target_frame = self.beats_frames[target_idx];
                    if let Some(a) = &self.audio {
                        let f = target_frame.min(a.frames.saturating_sub(1));
                        self.position_frames = f as f64;
                        self.handle.position_frames.store(f, Ordering::Release);
                    }
                }
            }
        }
    }

    /// Drain pending control commands without producing any audio.
    /// Called by the cpal master callback BEFORE `SyncEngine::process`
    /// so the sync engine sees the latest beats / play state when it
    /// computes phase error, then BEFORE `process()` so audio renders
    /// with the rate the sync engine just wrote.
    pub(crate) fn tick(&mut self) {
        self.drain_commands();
    }

    /// Render `out_frames` of stereo audio into `out` (length =
    /// out_frames × output_channels). Writes silence on empty / paused.
    pub(crate) fn process(&mut self, out: &mut [f32]) {
        // Always zero first — paused / no-track paths fall through.
        for s in out.iter_mut() {
            *s = 0.0;
        }
        // Note: tick() must be called by the caller before process().
        // We don't drain here so the sync engine can see post-tick state.

        let audio = match &self.audio {
            Some(a) => a,
            None => return,
        };
        if !self.handle.playing.load(Ordering::Acquire) {
            return;
        }
        let volume = f32::from_bits(self.handle.volume.load(Ordering::Acquire));
        let user_rate = f32::from_bits(self.handle.rate.load(Ordering::Acquire)) as f64;
        let eq_low = f32::from_bits(self.handle.eq_low.load(Ordering::Acquire));
        let eq_mid = f32::from_bits(self.handle.eq_mid.load(Ordering::Acquire));
        let eq_high = f32::from_bits(self.handle.eq_high.load(Ordering::Acquire));
        let filter_knob = f32::from_bits(self.handle.filter_knob.load(Ordering::Acquire));
        // Phase 2: linear-interpolated SRC × user-rate multiplier.
        // `audio.sample_rate / output_rate` handles 44.1 → 48 kHz format
        // conversion; multiplying by `user_rate` (1.0 = native) speeds up
        // or slows down playback. Pitch couples to speed (vinyl-style) —
        // Phase 2c may swap in a phase vocoder if pitch preservation is
        // needed.
        let rate_ratio = (audio.sample_rate as f64 / self.output_rate as f64) * user_rate;
        let total_frames = audio.frames as f64;
        let n_chan = self.output_channels as usize;
        let n_out_frames = out.len() / n_chan;

        let samples = audio.samples.as_ref();
        for i in 0..n_out_frames {
            if self.position_frames >= total_frames - 1.0 {
                // Ran off the end — auto-pause and emit silence.
                self.handle.playing.store(false, Ordering::Release);
                break;
            }
            let idx = self.position_frames as usize;
            let frac = (self.position_frames - idx as f64) as f32;
            // Linear interpolate between this stereo frame and the next.
            // samples is interleaved L,R,L,R,... so frame `idx` lives at
            // `samples[2*idx]` (L) and `samples[2*idx + 1]` (R).
            let i0 = idx * 2;
            let l = samples[i0] * (1.0 - frac) + samples[i0 + 2] * frac;
            let r = samples[i0 + 1] * (1.0 - frac) + samples[i0 + 3] * frac;
            // DSP chain: SRC → EQ → filter → volume → output.
            let (l_eq, r_eq) = self.eq.process_frame(l, r, eq_low, eq_mid, eq_high);
            let (l_dsp, r_dsp) = self.filter.process_frame(l_eq, r_eq, filter_knob);
            // Write to all output channels — channel 0 = L, 1 = R, 2+ = silence.
            let base = i * n_chan;
            if n_chan == 1 {
                // Mono device: sum L+R at -3 dB.
                out[base] = (l_dsp + r_dsp) * 0.5 * volume;
            } else {
                out[base] = l_dsp * volume;
                out[base + 1] = r_dsp * volume;
                // any extra channels stay zero (already cleared above)
            }
            self.position_frames += rate_ratio;
        }

        let pos_int = self.position_frames as u64;
        self.handle
            .position_frames
            .store(pos_int.min(audio.frames), Ordering::Release);
    }
}
