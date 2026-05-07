//! FFI bindings to the vendored QM-DSP beat tracker.
//!
//! Exposes Mixxx's exact beat-detection pipeline (CSD onset detection
//! → Davies-Plumbley TempoTrackV2 RCF → Viterbi → Ellis DP) as a single
//! synchronous Rust call. Same C++ code Mixxx ships — see
//! `vendor/qm_shim.cc` for the wrapper and `vendor/qm-dsp/` for the
//! upstream sources.
//!
//! License: QM-DSP is GPL-2; linking it makes the resulting DigTrax
//! binary GPL-2. Acceptable for the private repo per the plan.

use std::os::raw::{c_double, c_int};

#[link(name = "qmdsp")]
unsafe extern "C" {
    fn qmdsp_beat_analyze(
        mono_samples: *const c_double,
        n_frames: usize,
        sample_rate: c_int,
        out_bpm: *mut c_double,
        out_beats_seconds: *mut c_double,
        out_beats_capacity: usize,
        out_beats_count: *mut usize,
    ) -> c_int;
}

#[derive(Debug, Clone)]
pub struct QmBeats {
    pub bpm: f64,
    pub beats_seconds: Vec<f64>,
}

/// Run QM-DSP's beat tracker on a mono buffer. `samples` are in [-1, 1]
/// (the same convention Mixxx uses for its `CSAMPLE` after downmix).
///
/// Returns `Err` if the C side reports a problem (track too short, very
/// short DF, etc) — in those cases the caller should fall back gracefully.
pub fn analyze(samples: &[f64], sample_rate: u32) -> Result<QmBeats, String> {
    // Cap output capacity at "1 beat per second" — generous for any
    // realistic tempo (200 BPM = 3.3 beats/sec, but we headroom 1/sec
    // worth of slop just to be safe and round up).
    let dur_sec = samples.len() as f64 / sample_rate as f64;
    let cap = ((dur_sec * 5.0) as usize).max(64);
    let mut out_beats = vec![0f64; cap];
    let mut out_count: usize = 0;
    let mut out_bpm: f64 = 0.0;
    let rc = unsafe {
        qmdsp_beat_analyze(
            samples.as_ptr(),
            samples.len(),
            sample_rate as c_int,
            &mut out_bpm,
            out_beats.as_mut_ptr(),
            out_beats.capacity(),
            &mut out_count,
        )
    };
    if rc != 0 {
        return Err(format!("qmdsp_beat_analyze rc={rc}"));
    }
    out_beats.truncate(out_count);
    Ok(QmBeats { bpm: out_bpm, beats_seconds: out_beats })
}
