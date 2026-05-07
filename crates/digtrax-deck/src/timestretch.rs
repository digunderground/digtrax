// SPDX-License-Identifier: GPL-3.0-or-later
//
// Thin Rust wrapper around Rubber Band's C API. Used by the DJ deck
// engine for the per-deck Key Lock feature: when key lock is ON, the
// deck's audio runs through here so a tempo change preserves pitch.
//
// Lifetime: one [`TimeStretcher`] per deck, created at deck construction
// (so the audio callback never allocates) and reset on track load,
// seek, and key-lock OFF→ON transitions.

use std::ffi::c_void;

// Subset of `RubberBandOption` we actually use. Bitmask of flags
// passed to `rubberband_new`. See `rubberband-c.h` for the full list.
//
// We pick:
//   ProcessRealTime  — streaming mode (no offline study phase).
//   EngineFaster     — R2 engine, Mixxx's keylock default.
//   ThreadingNever   — single-threaded; we're already in our own
//                      audio callback, don't want RB spawning workers.
//   TransientsCrisp  — best preservation of percussive transients,
//                      important for DJ material.
//   PhaseLaminar     — default phase mode.
//   ChannelsApart    — process channels independently (true stereo
//                      preservation; CPU-cheaper than ChannelsTogether).
//   PitchHighSpeed   — fastest pitch scaling. We don't actually pitch
//                      scale (always 1.0), so this is just defensive.
//   WindowStandard   — default window length.
//   FormantShifted   — default; formants follow pitch (we keep pitch
//                      at 1.0 anyway).
//   DetectorCompound — default transient detector.
//   SmoothingOff     — default; on adds latency.
//
// Hex value below = OR of the picks above; computed manually rather
// than redeclaring all the option constants.
const OPTIONS: i32 = 0x00000001  // ProcessRealTime
                   | 0x00000000  // EngineFaster (= 0)
                   | 0x00010000  // ThreadingNever
                   | 0x00000000; // TransientsCrisp / PhaseLaminar /
                                 // ChannelsApart / PitchHighSpeed /
                                 // WindowStandard / FormantShifted /
                                 // DetectorCompound / SmoothingOff
                                 // are all 0 — defaults.

// FFI bindings to `rubberband-c.h`. We include only the calls used.
// The `RubberBandState` is an opaque pointer; we use `*mut c_void`.
#[link(name = "rubberband")]
extern "C" {
    fn rubberband_new(
        sample_rate: u32,
        channels: u32,
        options: i32,
        initial_time_ratio: f64,
        initial_pitch_scale: f64,
    ) -> *mut c_void;
    fn rubberband_delete(state: *mut c_void);
    fn rubberband_reset(state: *mut c_void);
    fn rubberband_set_time_ratio(state: *mut c_void, ratio: f64);
    fn rubberband_set_pitch_scale(state: *mut c_void, scale: f64);
    fn rubberband_set_max_process_size(state: *mut c_void, samples: u32);
    fn rubberband_get_samples_required(state: *const c_void) -> u32;
    fn rubberband_process(
        state: *mut c_void,
        input: *const *const f32,
        samples: u32,
        final_chunk: i32,
    );
    fn rubberband_available(state: *const c_void) -> i32;
    fn rubberband_retrieve(
        state: *const c_void,
        output: *const *mut f32,
        samples: u32,
    ) -> u32;
}

/// Stereo time-stretcher. Owns one Rubber Band state plus pre-sized
/// scratch buffers so the audio callback never allocates.
pub(crate) struct TimeStretcher {
    state: *mut c_void,
    sample_rate: u32,
    /// Pre-allocated planar input scratch [L, R]. Pushed into RB
    /// when the engine has source samples to feed.
    in_l: Vec<f32>,
    in_r: Vec<f32>,
    /// Pre-allocated planar output scratch [L, R]. Pulled from RB
    /// before being copied into the cpal callback's output slice.
    out_l: Vec<f32>,
    out_r: Vec<f32>,
}

// SAFETY: the `state` pointer is the only non-Send field. Rubber Band
// in `ThreadingNever` mode performs no thread synchronization itself;
// we are the only owner of the pointer and never share it. `Send` is
// fine; `Sync` would not be (concurrent calls into the same state are
// undefined). We don't impl Sync.
unsafe impl Send for TimeStretcher {}

impl TimeStretcher {
    /// Construct a stretcher for `sample_rate` Hz stereo audio.
    /// Allocates scratch for up to `max_chunk` frames per process call.
    pub(crate) fn new(sample_rate: u32, max_chunk: usize) -> Self {
        // SAFETY: rubberband_new returns null only on OOM. We assume
        // the audio engine startup is far enough from memory pressure
        // that this won't fail; if it does we panic at first use.
        let state = unsafe { rubberband_new(sample_rate, 2, OPTIONS, 1.0, 1.0) };
        debug_assert!(!state.is_null(), "rubberband_new returned null");
        unsafe {
            // Pre-tell RB the largest chunk we'll feed it so it can
            // size internal buffers at construction time, not on the
            // first push.
            rubberband_set_max_process_size(state, max_chunk as u32);
        }
        Self {
            state,
            sample_rate,
            in_l: vec![0.0; max_chunk],
            in_r: vec![0.0; max_chunk],
            out_l: vec![0.0; max_chunk],
            out_r: vec![0.0; max_chunk],
        }
    }

    /// Set the playback time ratio. `1.0` = no stretch. `>1.0` =
    /// output is longer (slower playback). `<1.0` = output is shorter
    /// (faster playback). Pitch is held constant.
    pub(crate) fn set_time_ratio(&mut self, ratio: f64) {
        unsafe { rubberband_set_time_ratio(self.state, ratio) };
    }

    /// Reset internal buffers. Call after seek, track load, or
    /// key-lock OFF→ON edge so we don't pick up where bypass left off.
    pub(crate) fn reset(&mut self) {
        unsafe {
            rubberband_reset(self.state);
            // Pitch always 1.0 for keylock; reset reaffirms it.
            rubberband_set_pitch_scale(self.state, 1.0);
        }
    }

    /// How many input frames RB wants right now to make progress.
    pub(crate) fn samples_required(&self) -> usize {
        (unsafe { rubberband_get_samples_required(self.state) }) as usize
    }

    /// How many output frames are ready. -1 = end-of-stream marker.
    pub(crate) fn available(&self) -> i32 {
        unsafe { rubberband_available(self.state) }
    }

    /// Push `frames` frames of input. The caller writes the planar
    /// data into the slices returned by [`scratch_in`].
    pub(crate) fn push(&mut self, frames: usize, final_chunk: bool) {
        debug_assert!(frames <= self.in_l.len());
        let l_ptr: *const f32 = self.in_l.as_ptr();
        let r_ptr: *const f32 = self.in_r.as_ptr();
        let channels: [*const f32; 2] = [l_ptr, r_ptr];
        unsafe {
            rubberband_process(
                self.state,
                channels.as_ptr(),
                frames as u32,
                if final_chunk { 1 } else { 0 },
            );
        }
    }

    /// Pull up to `out_frames` frames from RB into the internal output
    /// scratch. Returns how many frames were actually retrieved.
    pub(crate) fn pull(&mut self, out_frames: usize) -> usize {
        debug_assert!(out_frames <= self.out_l.len());
        let l_ptr: *mut f32 = self.out_l.as_mut_ptr();
        let r_ptr: *mut f32 = self.out_r.as_mut_ptr();
        let channels: [*mut f32; 2] = [l_ptr, r_ptr];
        let n = unsafe {
            rubberband_retrieve(self.state, channels.as_ptr(), out_frames as u32)
        };
        n as usize
    }

    /// Mutable access to the input scratch buffers — the caller fills
    /// these, then calls [`push`].
    pub(crate) fn scratch_in(&mut self) -> (&mut [f32], &mut [f32]) {
        (&mut self.in_l, &mut self.in_r)
    }

    /// Read access to the output scratch — populated by [`pull`].
    pub(crate) fn scratch_out(&self) -> (&[f32], &[f32]) {
        (&self.out_l, &self.out_r)
    }

    /// Sample rate this stretcher was constructed for. Engine uses
    /// this to detect rate changes that require a full rebuild.
    pub(crate) fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}

impl Drop for TimeStretcher {
    fn drop(&mut self) {
        if !self.state.is_null() {
            unsafe { rubberband_delete(self.state) };
            self.state = std::ptr::null_mut();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TimeStretcher;
    use std::f32::consts::TAU;

    fn sine_440hz(sample_rate: u32, secs: f32) -> Vec<f32> {
        let n = (sample_rate as f32 * secs) as usize;
        (0..n)
            .map(|i| (TAU * 440.0 * i as f32 / sample_rate as f32).sin())
            .collect()
    }

    /// Crude zero-crossing-rate estimator. Returns Hz.
    fn dominant_freq(samples: &[f32], sample_rate: u32) -> f32 {
        let mut crossings = 0usize;
        let mut prev = samples[0];
        for &s in &samples[1..] {
            if (prev <= 0.0 && s > 0.0) || (prev >= 0.0 && s < 0.0) {
                crossings += 1;
            }
            prev = s;
        }
        (crossings as f32) * (sample_rate as f32) / (2.0 * samples.len() as f32)
    }

    #[test]
    fn double_speed_preserves_pitch() {
        // 1s of 440 Hz sine, time_ratio = 0.5 → output ~0.5s at 440 Hz
        // (would be 880 Hz if pitch were coupled to tempo).
        let sr = 48_000u32;
        let max_chunk = 4096;
        let mut st = TimeStretcher::new(sr, max_chunk);
        st.set_time_ratio(0.5);

        let input = sine_440hz(sr, 1.0);

        let mut out = Vec::<f32>::new();
        let mut pushed = 0usize;
        while pushed < input.len() {
            let chunk = (input.len() - pushed).min(max_chunk);
            {
                let (sl, sr_) = st.scratch_in();
                sl[..chunk].copy_from_slice(&input[pushed..pushed + chunk]);
                sr_[..chunk].copy_from_slice(&input[pushed..pushed + chunk]);
            }
            let is_final = pushed + chunk == input.len();
            st.push(chunk, is_final);
            pushed += chunk;

            loop {
                let avail = st.available();
                if avail <= 0 { break; }
                let want = (avail as usize).min(max_chunk);
                let got = st.pull(want);
                if got == 0 { break; }
                let (sl, _) = st.scratch_out();
                out.extend_from_slice(&sl[..got]);
            }
        }
        loop {
            let avail = st.available();
            if avail <= 0 { break; }
            let got = st.pull((avail as usize).min(max_chunk));
            if got == 0 { break; }
            let (sl, _) = st.scratch_out();
            out.extend_from_slice(&sl[..got]);
        }

        let out_secs = out.len() as f32 / sr as f32;
        assert!(
            (0.40..0.60).contains(&out_secs),
            "expected ~0.5s output, got {out_secs:.3}s"
        );

        // Skip RB's startup latency (~50–100 ms) before measuring pitch.
        let skip = (sr as f32 * 0.1) as usize;
        if out.len() > skip + 2048 {
            let f = dominant_freq(&out[skip..], sr);
            assert!(
                (400.0..480.0).contains(&f),
                "expected pitch ~440 Hz, got {f:.1} Hz"
            );
        }
    }

    #[test]
    fn reset_does_not_panic() {
        let sr = 48_000u32;
        let mut st = TimeStretcher::new(sr, 1024);
        st.set_time_ratio(0.95);
        st.reset();
        assert_eq!(st.sample_rate(), sr);
    }
}
