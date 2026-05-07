//! [`MasterMixer`] — owns the cpal output stream and the two
//! [`DeckEngine`]s. The cpal callback closure is the only thread that
//! ever touches `DeckEngine` state; everything else goes through atomic
//! controls in [`MixerHandle`] and the per-deck [`DeckHandle`]s.
//!
//! ## Crossfader curve
//!
//! Crossfader value is `-1..+1`, matching the screenshot conventions
//! (`-1` = full deck A, `+1` = full deck B, `0` = center / equal-power).
//!
//! Equal-power crossfade keeps the *total perceived loudness* constant
//! across the sweep — at center both decks sit at `cos(π/4) = sin(π/4) ≈
//! 0.707`, summing to a constant power of 1.0 rather than dipping to
//! `0.5 + 0.5 = 1.0` linear (which would peak at the center).

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::sync::mpsc as stdmpsc;

use anyhow::{anyhow, Error};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, Stream, StreamConfig};

use crate::engine::{DeckEngine, DeckHandle};
use crate::sync::{SyncEngine, SyncHandle};
use crate::DeckId;

/// `Send`-able handle to the audio engine.
///
/// `cpal::Stream` is `!Send` on macOS (CoreAudio's audio unit can only
/// be touched on the thread that built it), and the WS task that owns
/// the surrounding `SocketContext` MUST be `Send` because axum runs it
/// across tokio worker threads. So we hide the stream entirely: it lives
/// in a dedicated `std::thread` that builds it, hands back the
/// `MixerHandle` via a sync channel, then parks indefinitely. The
/// stream is alive for as long as the parking thread is — i.e. for the
/// lifetime of the process. Quitting DigTrax cleans it up.
pub struct MasterMixer {
    handle: MixerHandle,
}

/// `Send + Clone` controller for the mixer. The WS layer holds one of
/// these and clones it for any thread that needs to drive playback.
#[derive(Clone)]
pub struct MixerHandle {
    deck_a: DeckHandle,
    deck_b: DeckHandle,
    /// Sync engine controller — see `SyncHandle::set_leader` /
    /// `set_sync`. The engine itself runs in the audio callback every
    /// buffer; this handle just lets the WS layer poke its atomics.
    sync: SyncHandle,
    /// f32 bits — crossfader in `[-1.0, 1.0]`. `0.0` = center.
    crossfader: Arc<AtomicU32>,
    /// f32 bits — master output gain in `[0.0, 1.0]` (linear). `1.0` = unity.
    master_gain: Arc<AtomicU32>,
    /// Output stream sample rate (Hz). Phase 2's beat-grid math needs this.
    output_rate: u32,
    /// Output stream channel count. 1, 2, or more (we only fill 0+1).
    output_channels: u16,
}

impl MasterMixer {
    /// Open the default audio output and start the stream. The cpal
    /// stream begins running immediately; both decks start in
    /// "no-track-loaded" silence.
    ///
    /// Internally spawns an OS thread that owns the cpal `Stream`. This
    /// is mandatory on macOS because CoreAudio's audio unit isn't
    /// `Send`; the surrounding tokio task can't carry it across awaits.
    /// The thread parks after handing back the `MixerHandle`.
    pub fn new() -> Result<MasterMixer, Error> {
        let (init_tx, init_rx) = stdmpsc::channel::<Result<MixerHandle, String>>();
        std::thread::Builder::new()
            .name("digtrax-deck-audio".into())
            .spawn(move || {
                match build_audio_thread() {
                    Ok((handle, _stream)) => {
                        // Hand the Send-able handle back to the caller.
                        let _ = init_tx.send(Ok(handle));
                        // Keep `_stream` alive (and therefore the audio
                        // callback running) until the process exits.
                        // park_timeout is fine here — cpal does the work
                        // on the OS audio thread regardless of what this
                        // thread does.
                        loop {
                            std::thread::park();
                        }
                    }
                    Err(e) => {
                        let _ = init_tx.send(Err(e.to_string()));
                    }
                }
            })
            .map_err(|e| anyhow!("spawn audio thread: {e}"))?;
        let handle = init_rx
            .recv()
            .map_err(|e| anyhow!("audio init channel: {e}"))?
            .map_err(|s| anyhow!("audio init: {s}"))?;
        Ok(MasterMixer { handle })
    }

    /// Cheap reference to the control surface.
    pub fn handle(&self) -> &MixerHandle {
        &self.handle
    }
}

/// Builds the cpal stream + handle pair. Runs ON the audio thread —
/// every cpal call (build_output_stream, play, the eventual drop) stays
/// on the same OS thread, satisfying CoreAudio's threading rules.
fn build_audio_thread() -> Result<(MixerHandle, Stream), Error> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| anyhow!("no default output device"))?;
    let supported = device
        .default_output_config()
        .map_err(|e| anyhow!("default_output_config: {e}"))?;
    let sample_rate = supported.sample_rate().0;
    let channels = supported.channels();
    let format = supported.sample_format();
    log::info!(
        "digtrax-deck: opening output device '{}' at {} Hz, {} ch, format {:?}",
        device.name().unwrap_or_else(|_| "?".into()),
        sample_rate,
        channels,
        format,
    );

    let (deck_a_engine, deck_a_handle) = DeckEngine::new(sample_rate, channels);
    let (deck_b_engine, deck_b_handle) = DeckEngine::new(sample_rate, channels);
    let (sync_engine, sync_handle) = SyncEngine::new();

    let crossfader = Arc::new(AtomicU32::new(0.0_f32.to_bits()));
    let master_gain = Arc::new(AtomicU32::new(1.0_f32.to_bits()));

    let cfg: StreamConfig = supported.config();
    let stream = build_stream(
        &device,
        &cfg,
        format,
        channels,
        deck_a_engine,
        deck_b_engine,
        sync_engine,
        crossfader.clone(),
        master_gain.clone(),
    )?;
    stream.play().map_err(|e| anyhow!("stream play: {e}"))?;

    let handle = MixerHandle {
        deck_a: deck_a_handle,
        deck_b: deck_b_handle,
        sync: sync_handle,
        crossfader,
        master_gain,
        output_rate: sample_rate,
        output_channels: channels,
    };
    Ok((handle, stream))
}

impl MixerHandle {
    /// Borrow the per-deck control handle.
    pub fn deck(&self, id: DeckId) -> &DeckHandle {
        match id {
            DeckId::A => &self.deck_a,
            DeckId::B => &self.deck_b,
        }
    }

    /// Borrow the sync controller.
    pub fn sync(&self) -> &SyncHandle {
        &self.sync
    }

    /// Set crossfader position. `-1.0` = full deck A, `+1.0` = full deck B,
    /// `0.0` = equal-power center.
    pub fn set_crossfader(&self, value: f32) {
        let v = value.clamp(-1.0, 1.0);
        self.crossfader.store(v.to_bits(), Ordering::Release);
    }

    /// Set master output gain. Clamped to `[0.0, 2.0]` (max 2.0 lets the
    /// user push +6 dB, matching Mixxx's master gain range).
    pub fn set_master_gain(&self, value: f32) {
        let v = value.clamp(0.0, 2.0);
        self.master_gain.store(v.to_bits(), Ordering::Release);
    }

    /// cpal output sample rate. Used by the WS push layer when converting
    /// frame indices to ms timestamps.
    pub fn output_rate(&self) -> u32 {
        self.output_rate
    }

    /// cpal output channel count.
    pub fn output_channels(&self) -> u16 {
        self.output_channels
    }
}

/// Build the cpal stream. Split out so the giant closure body doesn't
/// crowd `MasterMixer::new`. We support F32 native + I16/U16 conversions —
/// macOS/Linux/Windows all give us F32 in practice, so the I16/U16 paths
/// exist mainly for ALSA + the long-tail USB DACs.
fn build_stream(
    device: &cpal::Device,
    cfg: &StreamConfig,
    format: SampleFormat,
    channels: u16,
    mut deck_a: DeckEngine,
    mut deck_b: DeckEngine,
    sync: SyncEngine,
    crossfader: Arc<AtomicU32>,
    master_gain: Arc<AtomicU32>,
) -> Result<Stream, Error> {
    // Per-deck scratch buffers. Sized to accommodate the largest cpal
    // buffer we're likely to see; resized in-place if a host hands us
    // something bigger. Vec, not array, because cpal buffer size is
    // platform-defined at runtime.
    let mut buf_a: Vec<f32> = vec![0.0; 4096];
    let mut buf_b: Vec<f32> = vec![0.0; 4096];
    let n_chan = channels as usize;
    let err_fn = |e| log::error!("cpal stream error: {e}");

    let mix_into = move |out: &mut [f32]| {
        if buf_a.len() < out.len() {
            buf_a.resize(out.len(), 0.0);
        }
        if buf_b.len() < out.len() {
            buf_b.resize(out.len(), 0.0);
        }
        let buf_a_slice = &mut buf_a[..out.len()];
        let buf_b_slice = &mut buf_b[..out.len()];

        // Per-buffer pipeline:
        //   1. tick both decks      — drains commands (Load, Seek,
        //                              SetBeats, Play/Pause, etc.)
        //   2. sync.process         — reads each deck's beat_distance,
        //                              writes corrected rate atomic on
        //                              the follower
        //   3. process both decks   — generate audio at the rate the
        //                              sync engine just wrote
        // This ordering is sample-accurate: the rate change applies to
        // the same buffer whose phase produced it.
        deck_a.tick();
        deck_b.tick();
        sync.process(&deck_a, &deck_b);
        deck_a.process(buf_a_slice);
        deck_b.process(buf_b_slice);

        // Crossfader: -1..+1. Map to t in 0..1 and use cos/sin for equal
        // power. cf=-1 → t=0 → gain_a=cos(0)=1, gain_b=sin(0)=0. cf=0 →
        // t=0.5 → both at √½. cf=+1 → t=1 → gain_a=0, gain_b=1.
        let cf = f32::from_bits(crossfader.load(Ordering::Acquire)).clamp(-1.0, 1.0);
        let t = (cf + 1.0) * 0.5;
        let gain_a = (t * std::f32::consts::FRAC_PI_2).cos();
        let gain_b = (t * std::f32::consts::FRAC_PI_2).sin();
        let master = f32::from_bits(master_gain.load(Ordering::Acquire));

        for ((o, a), b) in out.iter_mut().zip(buf_a_slice.iter()).zip(buf_b_slice.iter()) {
            *o = (*a * gain_a + b * gain_b) * master;
        }
        let _ = n_chan; // suppress unused — channel routing happens inside DeckEngine::process
    };

    let stream = match format {
        SampleFormat::F32 => {
            let mut mix = mix_into;
            device
                .build_output_stream(
                    cfg,
                    move |out: &mut [f32], _| mix(out),
                    err_fn,
                    None,
                )
                .map_err(|e| anyhow!("build_output_stream f32: {e}"))?
        }
        SampleFormat::I16 => {
            let mut mix = mix_into;
            let mut tmp = vec![0f32; 4096];
            device
                .build_output_stream(
                    cfg,
                    move |out: &mut [i16], _| {
                        if tmp.len() < out.len() {
                            tmp.resize(out.len(), 0.0);
                        }
                        let scratch = &mut tmp[..out.len()];
                        mix(scratch);
                        for (s, &f) in out.iter_mut().zip(scratch.iter()) {
                            *s = (f.clamp(-1.0, 1.0) * (i16::MAX as f32)) as i16;
                        }
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| anyhow!("build_output_stream i16: {e}"))?
        }
        SampleFormat::U16 => {
            let mut mix = mix_into;
            let mut tmp = vec![0f32; 4096];
            device
                .build_output_stream(
                    cfg,
                    move |out: &mut [u16], _| {
                        if tmp.len() < out.len() {
                            tmp.resize(out.len(), 0.0);
                        }
                        let scratch = &mut tmp[..out.len()];
                        mix(scratch);
                        for (s, &f) in out.iter_mut().zip(scratch.iter()) {
                            // u16 expects unsigned with center at 0x8000.
                            let v = ((f.clamp(-1.0, 1.0) * 0.5 + 0.5)
                                * u16::MAX as f32) as u16;
                            *s = v;
                        }
                    },
                    err_fn,
                    None,
                )
                .map_err(|e| anyhow!("build_output_stream u16: {e}"))?
        }
        other => return Err(anyhow!("unsupported sample format: {:?}", other)),
    };
    Ok(stream)
}
