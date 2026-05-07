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
use crate::timestretch::TimeStretcher;

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
    /// Per-deck key lock — when true, tempo changes preserve pitch
    /// (RubberBand R2 in the audio path). When false, tempo and pitch
    /// couple (linear-interp resample, vinyl-style). Default: true.
    pub key_lock: bool,
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
    /// Translate the entire beat grid by `offset_frames` (signed).
    /// Used by the user-driven beat-grid corrections that mirror
    /// Mixxx's `beats_translate_half` / `beats_translate_earlier` /
    /// `beats_translate_later` controls. Negative = shift earlier.
    /// Out-of-range beats (i.e. negative or past the source) are
    /// dropped from the array.
    TranslateBeats(i64),
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
    /// Pitch-preserving time-stretcher for the per-deck KEY LOCK
    /// feature. Always allocated (so the audio callback never
    /// allocates); only fed when key_lock is ON and the rate is
    /// outside the safe-pass range.
    stretcher: TimeStretcher,
    /// Largest chunk we'll ever push into the stretcher in a single
    /// `process()` call. Must equal the value passed to
    /// `TimeStretcher::new` so the inner scratch buffers are big
    /// enough.
    stretch_max_chunk: usize,
    /// Edge-detection state for the key-lock atomic. When the toggle
    /// transitions OFF→ON we reset the stretcher (it has no useful
    /// internal state from the bypass period).
    last_key_lock_active: bool,
    /// Last `time_ratio` written to the stretcher. We only call
    /// `set_time_ratio` when this changes, so steady-state buffers
    /// don't pay the cost of an FFI call every callback.
    last_stretch_ratio: f64,
    /// Source-frame read head used by the keylock branch. Distinct
    /// from `position_frames` because the stretcher buffers ~50ms of
    /// input internally — we push ahead of the audible position. The
    /// public `position_frames` (read by the sync engine and by the
    /// frontend) tracks the AUDIBLE position, so both decks compare
    /// like-for-like regardless of whether they're in keylock or
    /// bypass. Kept in sync with `position_frames` outside the
    /// keylock branch and on every Seek / Load / BeatJump.
    read_head_frames: f64,
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
    /// Per-deck key lock toggle. `true` = pitch-preserving time-stretch
    /// (RubberBand). `false` = pitch-coupled resample (linear interp).
    /// Default: `true`. WS sets this via `set_key_lock`.
    key_lock: Arc<AtomicBool>,
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

    /// Toggle per-deck key lock. When `on`, tempo changes preserve pitch
    /// (RubberBand). When `off`, tempo and pitch couple (vinyl-style).
    /// Mixxx-equivalent: `keylock` per-deck control. Default state on
    /// every deck is `true`.
    pub fn set_key_lock(&self, on: bool) {
        self.key_lock.store(on, Ordering::Release);
    }

    /// Read current key-lock state. Pushed to frontend so reconnects
    /// sync the toggle UI without an extra round-trip.
    pub fn key_lock(&self) -> bool {
        self.key_lock.load(Ordering::Acquire)
    }

    /// Jump `n` beats from the current bracket beat. Negative = back.
    /// No-op if beats haven't been analyzed yet.
    pub fn beat_jump(&self, n: i32) {
        let _ = self.cmd_tx.try_send(DeckCommand::BeatJump(n));
    }

    /// Translate the beat grid by `offset_ms` (signed). Mirrors Mixxx's
    /// `beats_translate_*` family — for the user-driven kick-vs-snare
    /// fix (½ beat shift) and small earlier/later nudges.
    pub fn translate_beats_ms(&self, offset_ms: i64) {
        let sr = self.source_rate.load(std::sync::atomic::Ordering::Acquire);
        if sr == 0 { return; }
        let offset_frames = (offset_ms as f64 * sr as f64 / 1000.0).round() as i64;
        let _ = self.cmd_tx.try_send(DeckCommand::TranslateBeats(offset_frames));
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
            key_lock: self.key_lock.load(Ordering::Acquire),
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
        // Key lock defaults to ON (Mixxx-default-ish, also user spec).
        let key_lock = Arc::new(AtomicBool::new(true));
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
            key_lock,
        };
        // Pre-allocate the stretcher's scratch buffers for the largest
        // chunk we'll plausibly feed in one call. cpal callbacks at
        // 44.1/48 kHz are typically 256–1024 frames; we size for 4096
        // to leave headroom on systems with bigger callbacks.
        let stretch_max_chunk = 4096usize;
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
            stretcher: TimeStretcher::new(output_rate, stretch_max_chunk),
            stretch_max_chunk,
            last_key_lock_active: false,
            last_stretch_ratio: 1.0,
            read_head_frames: 0.0,
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

    /// Audio-thread side read of the rate atomic. The sync engine uses
    /// this on the leader so user-driven tempo changes on the leader
    /// (e.g. dragging the leader's tempo slider) propagate to the
    /// follower's base rate as well.
    pub(crate) fn current_rate(&self) -> f32 {
        f32::from_bits(self.handle.rate.load(Ordering::Acquire))
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
        // Keep the keylock read head in sync — sync engine snap-seeks
        // jump the audible position; we restart the stretcher so the
        // next push reads from the new location.
        self.read_head_frames = clamped;
        self.stretcher.reset();
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
                    self.read_head_frames = 0.0;
                    self.handle.position_frames.store(0, Ordering::Release);
                    self.handle.playing.store(false, Ordering::Release);
                    self.audio = Some(audio);
                    // Drop any audio still buffered inside the stretcher
                    // from a prior track — it'd play back at the wrong
                    // pitch otherwise.
                    self.stretcher.reset();
                    self.last_key_lock_active = false;
                }
                DeckCommand::Unload => {
                    self.audio = None;
                    self.position_frames = 0.0;
                    self.read_head_frames = 0.0;
                    self.handle.duration_frames.store(0, Ordering::Release);
                    self.handle.source_rate.store(0, Ordering::Release);
                    self.handle.position_frames.store(0, Ordering::Release);
                    self.handle.playing.store(false, Ordering::Release);
                    self.stretcher.reset();
                    self.last_key_lock_active = false;
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
                        self.read_head_frames = f as f64;
                        self.handle.position_frames.store(f, Ordering::Release);
                        // Stretcher's internal buffers refer to the old
                        // position; reset so we don't bleed into the new
                        // location.
                        self.stretcher.reset();
                    }
                }
                DeckCommand::SetBeats { beats_frames, bpm } => {
                    self.beats_frames = beats_frames;
                    self.file_bpm = bpm;
                }
                DeckCommand::TranslateBeats(offset_frames) => {
                    // Shift every beat by `offset_frames`. Drop beats
                    // that fall outside the source (negative or past
                    // the end). Frame range derived from loaded audio.
                    let max_frame = self.audio.as_ref()
                        .map(|a| a.frames as i64)
                        .unwrap_or(0);
                    if max_frame == 0 { continue; }
                    let mut translated: Vec<u64> =
                        Vec::with_capacity(self.beats_frames.len());
                    for &b in &self.beats_frames {
                        let shifted = b as i64 + offset_frames;
                        if shifted >= 0 && shifted < max_frame {
                            translated.push(shifted as u64);
                        }
                    }
                    self.beats_frames = translated;
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
                        self.read_head_frames = f as f64;
                        self.handle.position_frames.store(f, Ordering::Release);
                        // Same as Seek — old buffered audio is now stale.
                        self.stretcher.reset();
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

        // Lift everything we need out of `audio` so we don't keep an
        // immutable borrow of `self` while we mutate fields below.
        let (source_rate, total_frames_u64) = match &self.audio {
            Some(a) => (a.sample_rate, a.frames),
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
        let key_lock_on = self.handle.key_lock.load(Ordering::Acquire);
        let n_chan = self.output_channels as usize;
        let n_out_frames = out.len() / n_chan;
        if n_out_frames == 0 { return; }

        // Mixxx-style guard: at extreme rates RubberBand R2 sounds
        // bad enough that Mixxx force-disables keylock. We follow the
        // same thresholds (`enginebuffer.cpp:951–966`). At rate=1.0
        // the stretcher would just be added latency for no benefit.
        let stretch_active = key_lock_on
            && user_rate > 0.10
            && user_rate < 1.90
            && (user_rate - 1.0).abs() > 1e-4;

        // Detect OFF→ON edge (or rate-jump out of the bypass window
        // back into stretch range). In either case we reset the
        // stretcher and re-anchor the keylock read head to the
        // current audible position so the first push reads the right
        // source frames.
        if stretch_active && !self.last_key_lock_active {
            self.stretcher.reset();
            self.last_stretch_ratio = -1.0;
            self.read_head_frames = self.position_frames;
        }
        self.last_key_lock_active = stretch_active;

        let total_frames = total_frames_u64 as f64;
        // Source-to-output sample-rate ratio. Same in both branches.
        let src_step = source_rate as f64 / self.output_rate as f64;

        if stretch_active {
            // Tempo-only stretch: time_ratio = 1/user_rate. Pitch stays 1.0.
            let target_ratio = 1.0 / user_rate;
            if (target_ratio - self.last_stretch_ratio).abs() > 1e-9 {
                self.stretcher.set_time_ratio(target_ratio);
                self.last_stretch_ratio = target_ratio;
            }
            // Audible position advance per output frame: each output
            // sample represents one frame at the device rate; in source-
            // frame terms, that's `user_rate * src_step` (so playback
            // slowed to 0.95× advances ~5% fewer source frames per output
            // frame, exactly mirroring the linear-interp branch).
            let audible_step = user_rate * src_step;
            let mut written = 0usize;
            for _safety in 0..32 {
                if written >= n_out_frames { break; }

                let avail = self.stretcher.available();
                if avail < 0 { break; }   // End-of-stream.
                if avail > 0 {
                    let want = (n_out_frames - written)
                        .min(avail as usize)
                        .min(self.stretch_max_chunk);
                    let got = self.stretcher.pull(want);
                    if got > 0 {
                        for j in 0..got {
                            let (sl, sr) = self.stretcher.scratch_out();
                            let l = sl[j];
                            let r = sr[j];
                            let (l_eq, r_eq) =
                                self.eq.process_frame(l, r, eq_low, eq_mid, eq_high);
                            let (l_dsp, r_dsp) =
                                self.filter.process_frame(l_eq, r_eq, filter_knob);
                            let out_base = (written + j) * n_chan;
                            if n_chan == 1 {
                                out[out_base] = (l_dsp + r_dsp) * 0.5 * volume;
                            } else {
                                out[out_base] = l_dsp * volume;
                                out[out_base + 1] = r_dsp * volume;
                            }
                        }
                        // Advance AUDIBLE position per pulled output frame
                        // — this is what the sync engine reads. It must
                        // NOT include the stretcher's internal latency,
                        // otherwise a key-locked follower's beat_distance
                        // appears ~50ms ahead of where the audio actually
                        // is, and sync looks aligned while the kicks are
                        // visibly off.
                        self.position_frames += got as f64 * audible_step;
                        written += got;
                        continue;
                    }
                }

                // Need more input. Read from `read_head_frames` (which
                // runs ahead of `position_frames` by RB's internal
                // latency), then advance the read head only.
                let req = self.stretcher.samples_required();
                let push_n = req.max(64).min(self.stretch_max_chunk);
                let mut hit_end = false;
                {
                    let mut head = self.read_head_frames;
                    let (in_l, in_r) = self.stretcher.scratch_in();
                    for j in 0..push_n {
                        if head >= total_frames - 1.0 {
                            for k in j..push_n {
                                in_l[k] = 0.0;
                                in_r[k] = 0.0;
                            }
                            hit_end = true;
                            break;
                        }
                        let idx = head as usize;
                        let frac = (head - idx as f64) as f32;
                        let i0 = idx * 2;
                        let samples = self.audio.as_ref().unwrap().samples.as_ref();
                        in_l[j] = samples[i0] * (1.0 - frac) + samples[i0 + 2] * frac;
                        in_r[j] = samples[i0 + 1] * (1.0 - frac) + samples[i0 + 3] * frac;
                        head += src_step;
                    }
                    self.read_head_frames = head;
                }
                self.stretcher.push(push_n, hit_end);
                if hit_end {
                    // Mark end-of-stream so we don't keep feeding zeros.
                    // Actual playback stops once the stretcher drains.
                    self.handle.playing.store(false, Ordering::Release);
                }
            }
        } else {
            // Linear-interp path: vinyl-style pitch+tempo couple, or
            // keylock fallback at rate=1.0 / extreme rates.
            let rate_ratio =
                (source_rate as f64 / self.output_rate as f64) * user_rate;
            let samples = self.audio.as_ref().unwrap().samples.as_ref();
            for i in 0..n_out_frames {
                if self.position_frames >= total_frames - 1.0 {
                    self.handle.playing.store(false, Ordering::Release);
                    break;
                }
                let idx = self.position_frames as usize;
                let frac = (self.position_frames - idx as f64) as f32;
                let i0 = idx * 2;
                let l = samples[i0] * (1.0 - frac) + samples[i0 + 2] * frac;
                let r = samples[i0 + 1] * (1.0 - frac) + samples[i0 + 3] * frac;
                let (l_eq, r_eq) = self.eq.process_frame(l, r, eq_low, eq_mid, eq_high);
                let (l_dsp, r_dsp) = self.filter.process_frame(l_eq, r_eq, filter_knob);
                let out_base = i * n_chan;
                if n_chan == 1 {
                    out[out_base] = (l_dsp + r_dsp) * 0.5 * volume;
                } else {
                    out[out_base] = l_dsp * volume;
                    out[out_base + 1] = r_dsp * volume;
                }
                self.position_frames += rate_ratio;
            }
        }

        let pos_int = self.position_frames as u64;
        self.handle
            .position_frames
            .store(pos_int.min(total_frames_u64), Ordering::Release);
    }
}
