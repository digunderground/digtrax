//! Symphonia-based file → in-memory f32 stereo decoder.
//!
//! Phase 1 strategy: pre-decode the entire file to memory before Load
//! returns. A 5-min stereo 44.1 kHz f32 track is ~100 MB; for 2 decks
//! that's 200 MB resident, which is fine on any modern machine and
//! sidesteps the streaming-decode complexity (lock-free ring buffer +
//! decode worker thread). Phase 2 can swap in streaming if needed.
//!
//! Output is always **stereo interleaved f32 in [-1.0, 1.0]** regardless
//! of source format. Mono sources are duplicated to L+R; >2-channel
//! sources are downmixed to the first two channels (good enough for the
//! music DigTrax targets).

use std::fs::File;
use std::path::Path;
use std::sync::Arc;

use anyhow::{anyhow, Error};
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// A fully-decoded audio file, ready for sample-accurate playback.
#[derive(Clone)]
pub struct DecodedAudio {
    /// Stereo interleaved f32 samples. Length = `frames * 2`.
    /// `Arc` so handing the deck a new source is a cheap pointer swap;
    /// the audio thread can keep playing the previous source until the
    /// next callback iteration without us copying.
    pub samples: Arc<Vec<f32>>,
    /// Source sample rate. The audio thread compares this against the
    /// cpal output stream rate; if they differ, a resampler kicks in.
    pub sample_rate: u32,
    /// Total frames (`samples.len() / 2`).
    pub frames: u64,
}

impl DecodedAudio {
    /// Duration in milliseconds, computed from `frames` and `sample_rate`.
    pub fn duration_ms(&self) -> u64 {
        if self.sample_rate == 0 { return 0; }
        (self.frames as f64 * 1000.0 / self.sample_rate as f64).round() as u64
    }
}

/// Decode an audio file to in-memory stereo f32. Blocking — call from a
/// non-RT thread (the WS handler's spawn_blocking is fine).
pub fn decode_file(path: impl AsRef<Path>) -> Result<DecodedAudio, Error> {
    let path = path.as_ref();
    let file = File::open(path)
        .map_err(|e| anyhow!("open {}: {}", path.display(), e))?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());

    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| anyhow!("probe: {e}"))?;
    let mut format = probed.format;

    // Clone the track-specific bits we need so the rest of this function
    // can mutate `format` (next_packet is `&mut self`) without holding an
    // immutable borrow on its tracks list.
    let (track_id, sample_rate, n_chan_in, codec_params) = {
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| anyhow!("no supported audio track"))?;
        let sr = track
            .codec_params
            .sample_rate
            .ok_or_else(|| anyhow!("missing sample rate"))?;
        let n_chan = track
            .codec_params
            .channels
            .map(|c| c.count())
            .unwrap_or(2);
        (track.id, sr, n_chan, track.codec_params.clone())
    };

    let mut decoder = symphonia::default::get_codecs()
        .make(&codec_params, &DecoderOptions::default())
        .map_err(|e| anyhow!("make decoder: {e}"))?;

    // Pre-allocate. ~100 MB headroom for a 5-minute stereo 44.1 kHz f32
    // track; will grow if the file is longer.
    let mut samples: Vec<f32> = Vec::with_capacity(1 << 22);
    // Reusable converter buffer — allocated lazily once we know the
    // first packet's frame capacity.
    let mut sample_buf: Option<SampleBuffer<f32>> = None;

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymphoniaError::IoError(e))
                if e.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break;
            }
            Err(SymphoniaError::ResetRequired) => {
                // Format-level discontinuity (e.g. chained ogg). Rebuild the
                // decoder against the cached params and continue.
                decoder = symphonia::default::get_codecs()
                    .make(&codec_params, &DecoderOptions::default())
                    .map_err(|e| anyhow!("rebuild decoder: {e}"))?;
                continue;
            }
            Err(e) => return Err(anyhow!("next_packet: {e}")),
        };
        if packet.track_id() != track_id {
            continue;
        }
        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            // Skip individual decode errors — same defensiveness rodio uses
            // (a couple of corrupt frames shouldn't blow up the load).
            Err(SymphoniaError::DecodeError(_)) => continue,
            Err(SymphoniaError::IoError(_)) => continue,
            Err(e) => return Err(anyhow!("decode: {e}")),
        };

        // First packet — initialize the converter buffer to its max size.
        let buf = sample_buf.get_or_insert_with(|| {
            let spec = *decoded.spec();
            let dur = decoded.capacity() as u64;
            SampleBuffer::<f32>::new(dur, spec)
        });
        // copy_interleaved_ref handles any source format → f32 interleaved.
        buf.copy_interleaved_ref(decoded);
        let interleaved = buf.samples();

        // Append, downmixing/duplicating to enforce stereo output.
        match n_chan_in {
            1 => {
                // Mono → duplicate to L and R.
                samples.reserve(interleaved.len() * 2);
                for &s in interleaved {
                    samples.push(s);
                    samples.push(s);
                }
            }
            2 => {
                // Already stereo — append directly.
                samples.extend_from_slice(interleaved);
            }
            n => {
                // >2 channels — keep the first two (front L+R) and drop the rest.
                samples.reserve(interleaved.len() / n * 2);
                for chunk in interleaved.chunks_exact(n) {
                    samples.push(chunk[0]);
                    samples.push(chunk[1]);
                }
            }
        }
    }

    if samples.is_empty() {
        return Err(anyhow!("decoded zero samples"));
    }

    let frames = (samples.len() / 2) as u64;
    Ok(DecodedAudio {
        samples: Arc::new(samples),
        sample_rate,
        frames,
    })
}
