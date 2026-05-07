//! DJ-style filter knob: one knob, single sweep through LPF → bypass →
//! HPF. Pioneer `kill filter`-style.
//!
//! Knob convention: `[-1.0, 1.0]`.
//!   -1.0 → strongest LPF (cutoff ≈ 100 Hz)
//!    0.0 → bypass (pass-through, no filter)
//!   +1.0 → strongest HPF (cutoff ≈ 12 kHz)
//!
//! Cutoff sweeps logarithmically: doubling the knob's distance from
//! center doubles the perceived "strength". Logarithmic feels right
//! because pitch / frequency perception is logarithmic.
//!
//! Implementation: single 2nd-order Butterworth biquad per channel.
//! Coefficients recomputed only when the knob moves > 0.5% to keep
//! the steady-state callback cost to one biquad-tick per channel.

use crate::eq::{BiquadCoeffs, BiquadState, biquad_tick};

const FILTER_LPF_MAX_HZ: f32 = 22000.0;
const FILTER_LPF_MIN_HZ: f32 = 100.0;
const FILTER_HPF_MIN_HZ: f32 = 20.0;
const FILTER_HPF_MAX_HZ: f32 = 12000.0;
const FILTER_BYPASS_BAND: f32 = 0.02;

fn butter_lpf(sr: f32, fc: f32) -> BiquadCoeffs {
    use std::f32::consts::PI;
    let q = 1.0 / std::f32::consts::SQRT_2;
    let omega = 2.0 * PI * fc / sr;
    let cos_w = omega.cos();
    let alpha = omega.sin() / (2.0 * q);
    let a0 = 1.0 + alpha;
    BiquadCoeffs {
        b0: ((1.0 - cos_w) / 2.0) / a0,
        b1: (1.0 - cos_w) / a0,
        b2: ((1.0 - cos_w) / 2.0) / a0,
        a1: (-2.0 * cos_w) / a0,
        a2: (1.0 - alpha) / a0,
    }
}

fn butter_hpf(sr: f32, fc: f32) -> BiquadCoeffs {
    use std::f32::consts::PI;
    let q = 1.0 / std::f32::consts::SQRT_2;
    let omega = 2.0 * PI * fc / sr;
    let cos_w = omega.cos();
    let alpha = omega.sin() / (2.0 * q);
    let a0 = 1.0 + alpha;
    BiquadCoeffs {
        b0: ((1.0 + cos_w) / 2.0) / a0,
        b1: (-(1.0 + cos_w)) / a0,
        b2: ((1.0 + cos_w) / 2.0) / a0,
        a1: (-2.0 * cos_w) / a0,
        a2: (1.0 - alpha) / a0,
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Mode { Bypass, Lpf, Hpf }

pub(crate) struct FilterChain {
    sr: f32,
    last_knob: f32,
    mode: Mode,
    coef: BiquadCoeffs,
    state_l: BiquadState,
    state_r: BiquadState,
}

impl FilterChain {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sr: sample_rate as f32,
            last_knob: 0.0,
            mode: Mode::Bypass,
            coef: BiquadCoeffs::default(),
            state_l: BiquadState::default(),
            state_r: BiquadState::default(),
        }
    }

    fn recompute(&mut self, knob: f32) {
        self.last_knob = knob;
        if knob.abs() < FILTER_BYPASS_BAND {
            self.mode = Mode::Bypass;
            self.state_l = BiquadState::default();
            self.state_r = BiquadState::default();
            return;
        }
        if knob < 0.0 {
            // LPF: knob -1 → 100 Hz cutoff, knob 0 → 22 kHz (effectively
            // bypass). Logarithmic interpolation on cutoff frequency.
            let t = (-knob).clamp(0.0, 1.0); // 0..1 strength
            let log_lo = FILTER_LPF_MIN_HZ.ln();
            let log_hi = FILTER_LPF_MAX_HZ.ln();
            let cutoff = (log_hi + (log_lo - log_hi) * t).exp();
            self.coef = butter_lpf(self.sr, cutoff.clamp(20.0, self.sr * 0.45));
            self.mode = Mode::Lpf;
        } else {
            // HPF: knob +1 → 12 kHz cutoff (kill the lows), knob 0 →
            // 20 Hz (effectively bypass).
            let t = knob.clamp(0.0, 1.0);
            let log_lo = FILTER_HPF_MIN_HZ.ln();
            let log_hi = FILTER_HPF_MAX_HZ.ln();
            let cutoff = (log_lo + (log_hi - log_lo) * t).exp();
            self.coef = butter_hpf(self.sr, cutoff.clamp(20.0, self.sr * 0.45));
            self.mode = Mode::Hpf;
        }
    }

    /// Apply the filter in-place to one stereo frame.
    #[inline]
    pub fn process_frame(&mut self, l: f32, r: f32, knob: f32) -> (f32, f32) {
        if (knob - self.last_knob).abs() > 0.005 {
            self.recompute(knob);
        }
        match self.mode {
            Mode::Bypass => (l, r),
            _ => (
                biquad_tick(&self.coef, &mut self.state_l, l),
                biquad_tick(&self.coef, &mut self.state_r, r),
            ),
        }
    }
}
