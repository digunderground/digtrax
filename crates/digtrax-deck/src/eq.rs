//! 3-band EQ — DJ-mixer style High / Mid / Low shelf+peak topology.
//!
//! Topology (Mixxx's `BiquadFullKillEQEffect`):
//!   - LOW:  low-shelf @ 250 Hz
//!   - MID:  peaking  @ 1500 Hz, Q=0.5 (wide bell)
//!   - HIGH: high-shelf @ 5000 Hz
//!
//! Knob convention: each band's gain is a linear value in `[0.0, 2.0]`.
//! 0 = full kill (audibly silent at -inf dB; we clamp to -30 dB for
//! biquad stability). 1 = unity. 2 = +6 dB. Frontend maps the visual
//! knob (0..2 with center detent at 1) onto this directly.
//!
//! Per deck we keep one [`EqChain`] for left, one for right channel.
//! State is 2 floats per biquad × 3 biquads × 2 channels = 12 floats
//! plus the cached coefficients. Recomputed only when a knob changes.
//!
//! Cookbook biquad formulas: Robert Bristow-Johnson, "Audio EQ Cookbook".

#[derive(Clone, Copy, Default)]
pub(crate) struct BiquadCoeffs {
    pub b0: f32, pub b1: f32, pub b2: f32,
    pub a1: f32, pub a2: f32,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct BiquadState { pub s1: f32, pub s2: f32 }

#[inline]
pub(crate) fn biquad_tick(c: &BiquadCoeffs, s: &mut BiquadState, x: f32) -> f32 {
    // Direct form II transposed.
    let y = c.b0 * x + s.s1;
    s.s1 = c.b1 * x - c.a1 * y + s.s2;
    s.s2 = c.b2 * x - c.a2 * y;
    y
}

/// Map a 0..2 knob value to dB. 0 → -30 dB (effectively kill but
/// numerically safe), 1 → 0 dB, 2 → +6 dB.
fn knob_to_db(knob: f32) -> f32 {
    let k = knob.max(0.001); // -60 dB floor
    20.0 * k.log10()
}

/// Cookbook low-shelf @ `fc` with `gain_db` boost/cut, slope = 1
/// (Butterworth-ish). Returns normalized `BiquadCoeffs`.
fn low_shelf(sr: f32, fc: f32, gain_db: f32) -> BiquadCoeffs {
    use std::f32::consts::PI;
    let A = 10f32.powf(gain_db / 40.0);
    let w0 = 2.0 * PI * fc / sr;
    let cos_w = w0.cos();
    let sin_w = w0.sin();
    let s = 1.0;
    let alpha = sin_w / 2.0 * ((A + 1.0/A) * (1.0/s - 1.0) + 2.0).sqrt();
    let two_sqrtA_alpha = 2.0 * A.sqrt() * alpha;

    let b0 =     A * ((A + 1.0) - (A - 1.0) * cos_w + two_sqrtA_alpha);
    let b1 = 2.0*A * ((A - 1.0) - (A + 1.0) * cos_w);
    let b2 =     A * ((A + 1.0) - (A - 1.0) * cos_w - two_sqrtA_alpha);
    let a0 =          (A + 1.0) + (A - 1.0) * cos_w + two_sqrtA_alpha;
    let a1 =    -2.0*((A - 1.0) + (A + 1.0) * cos_w);
    let a2 =          (A + 1.0) + (A - 1.0) * cos_w - two_sqrtA_alpha;
    BiquadCoeffs { b0: b0/a0, b1: b1/a0, b2: b2/a0, a1: a1/a0, a2: a2/a0 }
}

/// Cookbook high-shelf @ `fc`.
fn high_shelf(sr: f32, fc: f32, gain_db: f32) -> BiquadCoeffs {
    use std::f32::consts::PI;
    let A = 10f32.powf(gain_db / 40.0);
    let w0 = 2.0 * PI * fc / sr;
    let cos_w = w0.cos();
    let sin_w = w0.sin();
    let s = 1.0;
    let alpha = sin_w / 2.0 * ((A + 1.0/A) * (1.0/s - 1.0) + 2.0).sqrt();
    let two_sqrtA_alpha = 2.0 * A.sqrt() * alpha;

    let b0 =     A * ((A + 1.0) + (A - 1.0) * cos_w + two_sqrtA_alpha);
    let b1 = -2.0*A* ((A - 1.0) + (A + 1.0) * cos_w);
    let b2 =     A * ((A + 1.0) + (A - 1.0) * cos_w - two_sqrtA_alpha);
    let a0 =          (A + 1.0) - (A - 1.0) * cos_w + two_sqrtA_alpha;
    let a1 =     2.0*((A - 1.0) - (A + 1.0) * cos_w);
    let a2 =          (A + 1.0) - (A - 1.0) * cos_w - two_sqrtA_alpha;
    BiquadCoeffs { b0: b0/a0, b1: b1/a0, b2: b2/a0, a1: a1/a0, a2: a2/a0 }
}

/// Cookbook peaking EQ (parametric bell) @ `fc`, `Q`.
fn peaking(sr: f32, fc: f32, q: f32, gain_db: f32) -> BiquadCoeffs {
    use std::f32::consts::PI;
    let A = 10f32.powf(gain_db / 40.0);
    let w0 = 2.0 * PI * fc / sr;
    let cos_w = w0.cos();
    let sin_w = w0.sin();
    let alpha = sin_w / (2.0 * q);

    let b0 = 1.0 + alpha * A;
    let b1 = -2.0 * cos_w;
    let b2 = 1.0 - alpha * A;
    let a0 = 1.0 + alpha / A;
    let a1 = -2.0 * cos_w;
    let a2 = 1.0 - alpha / A;
    BiquadCoeffs { b0: b0/a0, b1: b1/a0, b2: b2/a0, a1: a1/a0, a2: a2/a0 }
}

const LOW_FC:  f32 = 250.0;
const MID_FC:  f32 = 1500.0;
const MID_Q:   f32 = 0.5;
const HIGH_FC: f32 = 5000.0;

#[derive(Clone, Copy, Default)]
struct BandState { l: BiquadState, r: BiquadState }

/// Per-deck EQ. Two stereo channels, three bands in series. Coefficients
/// recomputed only when a knob actually moves (cheap dirty flag check).
pub(crate) struct EqChain {
    sr: f32,
    last_low_knob: f32,
    last_mid_knob: f32,
    last_high_knob: f32,
    low_coef: BiquadCoeffs,
    mid_coef: BiquadCoeffs,
    high_coef: BiquadCoeffs,
    low_state: BandState,
    mid_state: BandState,
    high_state: BandState,
}

impl EqChain {
    pub fn new(sample_rate: u32) -> Self {
        let mut me = EqChain {
            sr: sample_rate as f32,
            last_low_knob: 1.0,
            last_mid_knob: 1.0,
            last_high_knob: 1.0,
            low_coef: BiquadCoeffs::default(),
            mid_coef: BiquadCoeffs::default(),
            high_coef: BiquadCoeffs::default(),
            low_state: BandState::default(),
            mid_state: BandState::default(),
            high_state: BandState::default(),
        };
        me.recompute(1.0, 1.0, 1.0);
        me
    }

    fn recompute(&mut self, low_knob: f32, mid_knob: f32, high_knob: f32) {
        self.last_low_knob = low_knob;
        self.last_mid_knob = mid_knob;
        self.last_high_knob = high_knob;
        self.low_coef = low_shelf(self.sr, LOW_FC, knob_to_db(low_knob));
        self.mid_coef = peaking(self.sr, MID_FC, MID_Q, knob_to_db(mid_knob));
        self.high_coef = high_shelf(self.sr, HIGH_FC, knob_to_db(high_knob));
    }

    /// Apply the EQ in-place to a stereo frame `(l, r)`. Returns the
    /// processed pair. Audio thread calls this once per output frame
    /// after sample-rate conversion but before volume scaling.
    #[inline]
    pub fn process_frame(
        &mut self, l: f32, r: f32,
        low_knob: f32, mid_knob: f32, high_knob: f32,
    ) -> (f32, f32) {
        // Recompute coefs only when a knob actually moved (delta > 0.5%).
        // This keeps the steady-state CPU cost to one biquad-tick per
        // band per channel.
        let dirty = (low_knob - self.last_low_knob).abs() > 0.005
                 || (mid_knob - self.last_mid_knob).abs() > 0.005
                 || (high_knob - self.last_high_knob).abs() > 0.005;
        if dirty {
            self.recompute(low_knob, mid_knob, high_knob);
        }
        let l1 = biquad_tick(&self.low_coef,  &mut self.low_state.l,  l);
        let l2 = biquad_tick(&self.mid_coef,  &mut self.mid_state.l,  l1);
        let l3 = biquad_tick(&self.high_coef, &mut self.high_state.l, l2);
        let r1 = biquad_tick(&self.low_coef,  &mut self.low_state.r,  r);
        let r2 = biquad_tick(&self.mid_coef,  &mut self.mid_state.r,  r1);
        let r3 = biquad_tick(&self.high_coef, &mut self.high_state.r, r2);
        (l3, r3)
    }
}
