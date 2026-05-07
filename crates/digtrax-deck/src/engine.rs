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
    /// True if a track is loaded.
    pub loaded: bool,
}

/// Non-RT command sent from a `DeckHandle` to the audio thread.
pub(crate) enum DeckCommand {
    Load(DecodedAudio),
    Unload,
    Play,
    Pause,
    /// Seek to the given source-frame index.
    Seek(u64),
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
    /// Read-side handle — atomics this engine writes for outside readers.
    handle: DeckHandle,
}

/// Send-able controller for a deck. The WS layer holds these.
#[derive(Clone)]
pub struct DeckHandle {
    cmd_tx: Sender<DeckCommand>,
    /// f32 bits — user volume in [0.0, 1.0].
    volume: Arc<AtomicU32>,
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
}

impl DeckHandle {
    /// Set the deck volume. `value` is clamped to [0.0, 1.0].
    pub fn set_volume(&self, value: f32) {
        let v = value.clamp(0.0, 1.0);
        self.volume.store(v.to_bits(), Ordering::Release);
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
        let playing = Arc::new(AtomicBool::new(false));
        let position_frames = Arc::new(AtomicU64::new(0));
        let duration_frames = Arc::new(AtomicU64::new(0));
        let source_rate = Arc::new(AtomicU32::new(0));
        let handle = DeckHandle {
            cmd_tx,
            volume,
            playing,
            position_frames,
            duration_frames,
            source_rate,
        };
        let engine = DeckEngine {
            audio: None,
            position_frames: 0.0,
            output_rate,
            output_channels,
            cmd_rx,
            handle: handle.clone(),
        };
        (engine, handle)
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
            }
        }
    }

    /// Render `out_frames` of stereo audio into `out` (length =
    /// out_frames × output_channels). Writes silence on empty / paused.
    pub(crate) fn process(&mut self, out: &mut [f32]) {
        // Always zero first — paused / no-track paths fall through.
        for s in out.iter_mut() {
            *s = 0.0;
        }
        self.drain_commands();

        let audio = match &self.audio {
            Some(a) => a,
            None => return,
        };
        if !self.handle.playing.load(Ordering::Acquire) {
            return;
        }
        let volume = f32::from_bits(self.handle.volume.load(Ordering::Acquire));
        // Phase 1: linear-interpolated SRC. The "rate" here is purely
        // sample-rate conversion (source 44.1 kHz playing into a 48 kHz
        // output stream needs 44100/48000 ≈ 0.919 source-frames per output-
        // frame). Phase 2 will multiply this by the user's tempo control
        // and switch to a rubato resampler for higher quality.
        let rate_ratio = audio.sample_rate as f64 / self.output_rate as f64;
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
            // Write to all output channels — channel 0 = L, 1 = R, 2+ = silence.
            let base = i * n_chan;
            if n_chan == 1 {
                // Mono device: sum L+R at -3 dB.
                out[base] = (l + r) * 0.5 * volume;
            } else {
                out[base] = l * volume;
                out[base + 1] = r * volume;
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
