//! Kick-focused beat tracker tuned for 4-on-the-floor DJ music.
//!
//! Pipeline (carried over from `feature/dual-deck`'s working
//! synthetic-test version, adapted for the new `DecodedAudio` shape):
//!
//! 1. Mono mixdown (L+R / 2) of the already-decoded stereo f32 track.
//! 2. Zero-phase 30–150 Hz Butterworth bandpass (forward+reverse).
//! 3. Envelope follower: half-wave-rectify, square, 15 Hz one-pole
//!    smoother, decimate to 200 Hz envelope rate.
//! 4. Period via biased ACF + Gaussian tempo prior at 128 BPM (σ=30).
//!    Parabolic interp around the peak gives sub-frame BPM precision.
//! 5. First-beat phase via comb-sum, then snapped to the strongest
//!    envelope peak in the first 4 beats (so beat 1 lands on a real
//!    audible kick, not a synthetic phase).
//! 6. Beat sequence emission: predicted beats at `first_beat + n·period`,
//!    each snapped to the nearest envelope peak within ±period/4. Where
//!    no kick exists (build-up bars, breakdowns) the beat stays at the
//!    predicted slot — the output never has gaps, which keeps downstream
//!    sync math simple.
//!
//! Independently, a 3-band STFT spectrum is computed for the waveform
//! renderer (low / mid / high RGB triplets, normalised to [0, 1]).
//!
//! All synthetic-click tests at 100/120/124/128/140/150/170 BPM passed
//! within ±0.5 BPM on the prior branch — see `tests` module below.

use anyhow::Error;
use realfft::RealFftPlanner;
use std::f64::consts::PI;

use crate::decode::DecodedAudio;

const KICK_LO_HZ: f64 = 30.0;
const KICK_HI_HZ: f64 = 150.0;
const ENV_RATE: f64 = 200.0;
const ENV_LPF_HZ: f64 = 15.0;

const BPM_MIN: f64 = 70.0;
const BPM_MAX: f64 = 200.0;
const BPM_PRIOR_MEAN: f64 = 128.0;
const BPM_PRIOR_SIGMA: f64 = 30.0;

const STFT_FFT: usize = 2048;
const STFT_HOP: usize = 512;

/// Beat-tracker output. `bpm` and `first_beat_ms` are derived from
/// `beats_ms` by averaging across the whole track for sub-frame precision.
#[derive(Debug, Clone)]
pub struct BeatAnalysis {
    pub bpm: f32,
    pub first_beat_ms: u64,
    pub confidence: f32,
    /// Every detected beat, in milliseconds (file-time). Drives the
    /// waveform's beat-grid markers — visual grid = exactly the beats
    /// the algorithm found, no synthetic extrapolation.
    pub beats_ms: Vec<u64>,
    /// Per-bin 3-band amplitude triplets (low/mid/high), normalised to
    /// [0, 1]. Length = `duration_sec * bars_per_sec`.
    pub spectrum_bars: Vec<[f32; 3]>,
}

/// Run the full analysis on a decoded track. Blocking — call from
/// `tokio::task::spawn_blocking`.
pub fn analyze(audio: &DecodedAudio, bars_per_sec: f32) -> Result<BeatAnalysis, Error> {
    let bars_per_sec = if bars_per_sec.is_finite() && bars_per_sec > 0.0 { bars_per_sec } else { 10.0 };
    let sample_rate = audio.sample_rate as f64;

    // Mix interleaved stereo to mono f64 for the DSP. f64 throughout the
    // beat tracker — biquads compound rounding error if we drop to f32.
    let n_frames = audio.frames as usize;
    if n_frames < (sample_rate * 2.0) as usize {
        return Ok(BeatAnalysis {
            bpm: 0.0, first_beat_ms: 0, confidence: 0.0,
            beats_ms: vec![], spectrum_bars: vec![],
        });
    }
    let mut mono = Vec::<f64>::with_capacity(n_frames);
    let samples = audio.samples.as_ref();
    for f in 0..n_frames {
        let i = f * 2;
        mono.push((samples[i] as f64 + samples[i + 1] as f64) * 0.5);
    }

    let (bpm, first_beat_ms, beats_ms, confidence) = detect_kicks(&mono, sample_rate);
    let spectrum_bars = compute_spectrum(&mono, sample_rate, bars_per_sec);
    Ok(BeatAnalysis { bpm, first_beat_ms, confidence, beats_ms, spectrum_bars })
}

// =============================================================================
// Kick tracker.
// =============================================================================

fn detect_kicks(mono: &[f64], sample_rate: f64) -> (f32, u64, Vec<u64>, f32) {
    let bp = bandpass_zero_phase(mono, sample_rate, KICK_LO_HZ, KICK_HI_HZ);
    let env = envelope_follower(&bp, sample_rate, ENV_RATE, ENV_LPF_HZ);
    if env.len() < (ENV_RATE * 1.0) as usize {
        return (0.0, 0, vec![], 0.0);
    }

    let period_frames = estimate_period(&env, ENV_RATE);
    if !period_frames.is_finite() || period_frames <= 1.0 {
        return (0.0, 0, vec![], 0.0);
    }
    let bpm = (60.0 * ENV_RATE / period_frames) as f32;

    let first_beat_frame = find_first_beat(&env, period_frames);

    let snap_radius = (period_frames * 0.25).max(2.0);
    let mut beats: Vec<f64> = Vec::new();
    let mut t = first_beat_frame;
    let n_env = env.len();
    while (t as usize) < n_env {
        let t_int = t as usize;
        let lo = (t_int.saturating_sub(snap_radius as usize)).min(n_env - 1);
        let hi = (t_int + snap_radius as usize).min(n_env - 1);
        let mut best_i = t_int;
        let mut best_v = f64::NEG_INFINITY;
        for i in lo..=hi {
            if env[i] > best_v { best_v = env[i]; best_i = i; }
        }
        beats.push(best_i as f64);
        t += period_frames;
    }

    if beats.is_empty() {
        return (0.0, 0, vec![], 0.0);
    }

    // Re-derive BPM from the snapped beats — averaging across the track
    // gives sub-frame precision (individual beat indices are integer
    // envelope frames; the average is not).
    let bpm_refined = if beats.len() >= 2 {
        let span = beats[beats.len() - 1] - beats[0];
        let avg_p = span / (beats.len() as f64 - 1.0);
        (60.0 * ENV_RATE / avg_p) as f32
    } else { bpm };

    let max_env = env.iter().cloned().fold(0f64, f64::max).max(1e-9);
    let strong = beats.iter().filter(|&&b| env[b as usize] >= 0.3 * max_env).count();
    let confidence = (strong as f32 / beats.len() as f32).clamp(0.0, 1.0);

    let to_ms = |frame: f64| -> u64 {
        ((frame / ENV_RATE) * 1000.0).round().max(0.0) as u64
    };
    let beats_ms: Vec<u64> = beats.iter().map(|&b| to_ms(b)).collect();
    let first_beat_ms = beats_ms.first().copied().unwrap_or(0);

    log::info!(
        "beats: {:.2} BPM ({:.2} refined), {} beats, confidence {:.2}",
        bpm, bpm_refined, beats_ms.len(), confidence,
    );

    (bpm_refined, first_beat_ms, beats_ms, confidence)
}

fn estimate_period(env: &[f64], env_rate: f64) -> f64 {
    let n = env.len();
    let lag_min = (env_rate * 60.0 / BPM_MAX).floor() as usize;
    let lag_max = ((env_rate * 60.0 / BPM_MIN).ceil() as usize).min(n - 1);
    if lag_max <= lag_min + 2 { return 0.0; }

    let acf = autocorrelate_via_fft(env, lag_max + 4);
    let mut best_lag = lag_min;
    let mut best_score = 0f64;
    for lag in lag_min..=lag_max {
        let bpm_at = 60.0 * env_rate / lag as f64;
        let prior = (-((bpm_at - BPM_PRIOR_MEAN).powi(2))
                     / (2.0 * BPM_PRIOR_SIGMA * BPM_PRIOR_SIGMA)).exp();
        let score = acf[lag] * prior;
        if score > best_score { best_score = score; best_lag = lag; }
    }

    if best_lag > lag_min && best_lag < lag_max {
        let y0 = acf[best_lag - 1];
        let y1 = acf[best_lag];
        let y2 = acf[best_lag + 1];
        let denom = y0 - 2.0 * y1 + y2;
        let delta = if denom.abs() > 1e-12 { 0.5 * (y0 - y2) / denom } else { 0.0 };
        best_lag as f64 + delta.clamp(-1.0, 1.0)
    } else {
        best_lag as f64
    }
}

fn find_first_beat(env: &[f64], period_frames: f64) -> f64 {
    let p = period_frames.round() as usize;
    if p == 0 || p >= env.len() { return 0.0; }

    let mut best_phase = 0usize;
    let mut best_score = -1f64;
    for phase in 0..p {
        let mut sum = 0f64;
        let mut k = 0usize;
        loop {
            let idx = phase + k * p;
            if idx >= env.len() { break; }
            sum += env[idx];
            k += 1;
        }
        if sum > best_score { best_score = sum; best_phase = phase; }
    }

    let mut anchor_frame = best_phase;
    let mut anchor_strength = -1f64;
    let half_window = (p / 4).max(1);
    for k in 0..4 {
        let center = best_phase + k * p;
        if center >= env.len() { break; }
        let lo = center.saturating_sub(half_window);
        let hi = (center + half_window).min(env.len() - 1);
        for i in lo..=hi {
            if env[i] > anchor_strength {
                anchor_strength = env[i];
                anchor_frame = i;
            }
        }
    }
    (anchor_frame % p) as f64
}

// =============================================================================
// DSP building blocks (cookbook biquad + 1-pole envelope follower + FFT ACF).
// =============================================================================

#[derive(Clone, Copy)]
struct Biquad { b0: f64, b1: f64, b2: f64, a1: f64, a2: f64 }

fn butter_hpf(sr: f64, fc: f64) -> Biquad {
    let q = 1.0 / std::f64::consts::SQRT_2;
    let omega = 2.0 * PI * fc / sr;
    let cos_w = omega.cos();
    let alpha = omega.sin() / (2.0 * q);
    let a0 = 1.0 + alpha;
    Biquad {
        b0: ((1.0 + cos_w) / 2.0) / a0,
        b1: (-(1.0 + cos_w)) / a0,
        b2: ((1.0 + cos_w) / 2.0) / a0,
        a1: (-2.0 * cos_w) / a0,
        a2: (1.0 - alpha) / a0,
    }
}

fn butter_lpf(sr: f64, fc: f64) -> Biquad {
    let q = 1.0 / std::f64::consts::SQRT_2;
    let omega = 2.0 * PI * fc / sr;
    let cos_w = omega.cos();
    let alpha = omega.sin() / (2.0 * q);
    let a0 = 1.0 + alpha;
    Biquad {
        b0: ((1.0 - cos_w) / 2.0) / a0,
        b1: (1.0 - cos_w) / a0,
        b2: ((1.0 - cos_w) / 2.0) / a0,
        a1: (-2.0 * cos_w) / a0,
        a2: (1.0 - alpha) / a0,
    }
}

#[inline]
fn biquad_tick(b: &Biquad, s: &mut [f64; 2], x: f64) -> f64 {
    let y = b.b0 * x + s[0];
    s[0] = b.b1 * x - b.a1 * y + s[1];
    s[1] = b.b2 * x - b.a2 * y;
    y
}

/// Cascaded HPF→LPF biquads run forward then reverse → zero group delay.
/// Critical for the beat tracker: kick onsets stay sample-aligned with
/// the source audio (otherwise yellow markers drift past the kick).
fn bandpass_zero_phase(input: &[f64], sr: f64, lo: f64, hi: f64) -> Vec<f64> {
    let hp = butter_hpf(sr, lo);
    let lp = butter_lpf(sr, hi);
    let apply = |buf: &[f64]| -> Vec<f64> {
        let mut hps = [0f64; 2];
        let mut lps = [0f64; 2];
        buf.iter().map(|&x| biquad_tick(&lp, &mut lps, biquad_tick(&hp, &mut hps, x))).collect()
    };
    let fwd = apply(input);
    let rev_in: Vec<f64> = fwd.iter().rev().copied().collect();
    let rev_out = apply(&rev_in);
    rev_out.into_iter().rev().collect()
}

fn envelope_follower(bp: &[f64], sr_in: f64, sr_out: f64, lpf_hz: f64) -> Vec<f64> {
    let rc = 1.0 / (2.0 * PI * lpf_hz);
    let dt = 1.0 / sr_in;
    let alpha = dt / (rc + dt);
    let mut env = Vec::with_capacity(bp.len());
    let mut y = 0f64;
    for &x in bp {
        let r = if x > 0.0 { x * x } else { 0.0 };
        y += alpha * (r - y);
        env.push(y);
    }
    let factor = (sr_in / sr_out).round().max(1.0) as usize;
    env.into_iter().step_by(factor).collect()
}

fn autocorrelate_via_fft(x: &[f64], max_lag: usize) -> Vec<f64> {
    let n = x.len();
    let pad = (n + max_lag).next_power_of_two();
    let mut planner = RealFftPlanner::<f64>::new();
    let fft = planner.plan_fft_forward(pad);
    let ifft = planner.plan_fft_inverse(pad);
    let mut buf = vec![0f64; pad];
    buf[..n].copy_from_slice(x);
    let mut spec = fft.make_output_vec();
    if fft.process(&mut buf, &mut spec).is_err() { return vec![0f64; max_lag + 1]; }
    for c in spec.iter_mut() {
        *c = realfft::num_complex::Complex::new(c.norm_sqr(), 0.0);
    }
    let mut out = vec![0f64; pad];
    if ifft.process(&mut spec, &mut out).is_err() { return vec![0f64; max_lag + 1]; }
    let scale = 1.0 / pad as f64;
    out.truncate((max_lag + 1).min(pad));
    for v in out.iter_mut() {
        *v *= scale;
        if *v < 0.0 { *v = 0.0; }
    }
    out
}

// =============================================================================
// 3-band STFT spectrum for the waveform renderer.
// =============================================================================

fn compute_spectrum(mono: &[f64], sample_rate: f64, bars_per_sec: f32) -> Vec<[f32; 3]> {
    let duration_sec = mono.len() as f64 / sample_rate;
    let bars = ((duration_sec as f32 * bars_per_sec).ceil() as usize).max(1);

    let mut window = [0f32; STFT_FFT];
    for i in 0..STFT_FFT {
        window[i] = (0.5 - 0.5 * (2.0 * std::f32::consts::PI * i as f32 / STFT_FFT as f32).cos()) as f32;
    }

    let mut planner = RealFftPlanner::<f32>::new();
    let r2c = planner.plan_fft_forward(STFT_FFT);
    let mut input = r2c.make_input_vec();
    let mut output = r2c.make_output_vec();
    let n_bins = output.len();

    let bin_hz = sample_rate as f32 / STFT_FFT as f32;
    let lo_end = ((258.0 / bin_hz).round() as usize).clamp(1, n_bins);
    let mid_end = ((4000.0 / bin_hz).round() as usize).clamp(lo_end + 1, n_bins);

    if mono.len() < STFT_FFT * 2 {
        return vec![[0.0, 0.0, 0.0]; bars];
    }

    let n_frames = (mono.len() - STFT_FFT) / STFT_HOP + 1;
    let mut frame_lo = vec![0f32; n_frames];
    let mut frame_mid = vec![0f32; n_frames];
    let mut frame_hi = vec![0f32; n_frames];

    for f in 0..n_frames {
        let start = f * STFT_HOP;
        for i in 0..STFT_FFT {
            input[i] = mono[start + i] as f32 * window[i];
        }
        if r2c.process(&mut input, &mut output).is_err() { break; }
        let mut lo_e = 0f32;
        let mut mid_e = 0f32;
        let mut hi_e = 0f32;
        for (i, c) in output.iter().enumerate() {
            let e = c.re * c.re + c.im * c.im;
            if i < lo_end { lo_e += e; }
            else if i < mid_end { mid_e += e; }
            else { hi_e += e; }
        }
        frame_lo[f] = lo_e;
        frame_mid[f] = mid_e;
        frame_hi[f] = hi_e;
    }

    let mut max_lo = 1f32;
    let mut max_mid = 1f32;
    let mut max_hi = 1f32;
    for f in 0..n_frames {
        if frame_lo[f] > max_lo { max_lo = frame_lo[f]; }
        if frame_mid[f] > max_mid { max_mid = frame_mid[f]; }
        if frame_hi[f] > max_hi { max_hi = frame_hi[f]; }
    }
    max_lo = max_lo.sqrt();
    max_mid = max_mid.sqrt();
    max_hi = max_hi.sqrt();

    let mut spectrum_bars = Vec::with_capacity(bars);
    for b in 0..bars {
        let f0 = (b * n_frames) / bars;
        let f1 = ((b + 1) * n_frames / bars).max(f0 + 1).min(n_frames);
        let span = (f1 - f0).max(1) as f32;
        let mut sum_lo = 0f32;
        let mut sum_mid = 0f32;
        let mut sum_hi = 0f32;
        for f in f0..f1 {
            sum_lo += frame_lo[f];
            sum_mid += frame_mid[f];
            sum_hi += frame_hi[f];
        }
        let lo = ((sum_lo / span).sqrt() / max_lo * 1.5).clamp(0.0, 1.0);
        let mid = ((sum_mid / span).sqrt() / max_mid * 1.5).clamp(0.0, 1.0);
        let hi = ((sum_hi / span).sqrt() / max_hi * 1.5).clamp(0.0, 1.0);
        spectrum_bars.push([lo, mid, hi]);
    }
    spectrum_bars
}

// =============================================================================
// Synthetic-click tests. These passed at sub-0.5 BPM accuracy across
// 100/120/124/128/140/150/170 BPM in the prior implementation; carrying
// them across as a regression gate.
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn click_track(bpm: f64, seconds: f64, sr: f64) -> Vec<f64> {
        let total = (sr * seconds) as usize;
        let period = (60.0 / bpm * sr) as usize;
        let mut out = vec![0f64; total];
        let mut t = period / 2;
        while t < total {
            for i in 0..((0.08 * sr) as usize) {
                if t + i >= total { break; }
                let phase = 2.0 * PI * 60.0 * i as f64 / sr;
                let env = (-(i as f64) / (sr * 0.02)).exp();
                out[t + i] += phase.sin() * env * 0.6;
            }
            t += period;
        }
        out
    }

    fn detect_bpm(target_bpm: f64) -> f32 {
        let mono = click_track(target_bpm, 30.0, 44_100.0);
        let (b, _, _, _) = detect_kicks(&mono, 44_100.0);
        b
    }

    #[test] fn detects_100() { let b = detect_bpm(100.0); assert!((b - 100.0).abs() < 0.5, "got {b}"); }
    #[test] fn detects_120() { let b = detect_bpm(120.0); assert!((b - 120.0).abs() < 0.5, "got {b}"); }
    #[test] fn detects_124() { let b = detect_bpm(124.0); assert!((b - 124.0).abs() < 0.5, "got {b}"); }
    #[test] fn detects_128() { let b = detect_bpm(128.0); assert!((b - 128.0).abs() < 0.5, "got {b}"); }
    #[test] fn detects_140() { let b = detect_bpm(140.0); assert!((b - 140.0).abs() < 0.5, "got {b}"); }
    #[test] fn detects_150() { let b = detect_bpm(150.0); assert!((b - 150.0).abs() < 0.5, "got {b}"); }
    #[test] fn detects_170() { let b = detect_bpm(170.0); assert!((b - 170.0).abs() < 0.5, "got {b}"); }
}
