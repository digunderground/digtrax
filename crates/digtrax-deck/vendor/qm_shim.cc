// SPDX-License-Identifier: GPL-2.0-or-later
//
// DigTrax — DJ Mode beat detection shim.
// Copyright (C) 2024-2026 Adam Tout
//
// Pipeline mirrors Mixxx's analyzerqueenmarybeats.cpp (Mixxx is
// © 2001-2026 The Mixxx Development Team, GPL-2-or-later) which
// drives the vendored QM-DSP source under vendor/qm-dsp/ (QM-DSP is
// © 2005-2018 Centre for Digital Music, Queen Mary, University of
// London, GPL-2-or-later).
//
// This file is part of DigTrax. DigTrax is free software: you can
// redistribute it and/or modify it under the terms of the GNU General
// Public License as published by the Free Software Foundation, either
// version 3 of the License, or (at your option) any later version.
// See the LICENSE file at the project root for the full text. The
// QM-DSP COPYING file (GPL-2) lives at vendor/qm-dsp/COPYING.
//
// Thin C wrapper around QM-DSP that runs Mixxx's exact beat-detection
// pipeline (see research/mixxx/src/analyzer/plugins/analyzerqueenmarybeats.cpp).
//
// Steps, line-for-line with Mixxx:
//   1. step_size = (int)(sample_rate * 0.01161)        // 11.61 ms
//   2. window_size = nextPowerOfTwo(sample_rate / 50)  // ~1024 @ 44.1kHz
//   3. DFConfig { DF_COMPLEXSD, stepSize, frameLength=windowSize,
//                 dbRise=3, no whitening }
//   4. DetectionFunction df(config);
//   5. Centered windowing: first window has windowSize/2 zeros prepended,
//      second half is samples[0..stepSize], then sliding by stepSize.
//      We achieve this by pre-padding the input with windowSize/2 zeros
//      front and back.
//   6. Each window → df.processTimeDomain(buf) → one DF value
//   7. Trim trailing zeros + skip first 2 frames (Mixxx convention)
//   8. TempoTrackV2 tt(sample_rate, step_size);
//   9. tt.calculateBeatPeriod(df, beat_period) where beat_period is sized
//      df.size()/128 + 1
//  10. tt.calculateBeats(df, beat_period, beats)
//  11. Beat sample positions: beats[i] * step_size + step_size/2
//
// The returned BPM is computed from the median inter-beat interval,
// matching how Mixxx surfaces it for sync.

#include "dsp/onsets/DetectionFunction.h"
#include "dsp/tempotracking/TempoTrackV2.h"
#include "maths/MathUtilities.h"

#include <algorithm>
#include <cmath>
#include <cstddef>
#include <cstdint>
#include <vector>

extern "C" int qmdsp_beat_analyze(
    const double* mono_samples,
    size_t n_frames,
    int sample_rate,
    double* out_bpm,
    double* out_beats_seconds,
    size_t out_beats_capacity,
    size_t* out_beats_count
) {
    if (!mono_samples || !out_bpm || !out_beats_seconds || !out_beats_count) return 1;
    if (sample_rate <= 0 || n_frames < (size_t)sample_rate * 2) return 2;

    *out_beats_count = 0;
    *out_bpm = 0.0;

    const float kStepSecs = 0.01161f;
    const int step_size = static_cast<int>(sample_rate * kStepSecs);
    const int window_size = MathUtilities::nextPowerOfTwo(sample_rate / 50);
    if (step_size <= 0 || window_size <= step_size) return 3;
    const size_t half_window = static_cast<size_t>(window_size) / 2;

    // Mixxx's DFConfig — DF_COMPLEXSD with no whitening.
    DFConfig cfg;
    cfg.DFType = DF_COMPLEXSD;
    cfg.stepSize = step_size;
    cfg.frameLength = window_size;
    cfg.dbRise = 3;
    cfg.adaptiveWhitening = false;
    cfg.whiteningRelaxCoeff = -1;
    cfg.whiteningFloor = -1;
    DetectionFunction df(cfg);

    // Pre-pad with half_window zeros front and back. Equivalent to
    // Mixxx's DownmixAndOverlapHelper centered-windowing trick — frame
    // index 0 corresponds to audio sample 0, not sample windowSize/2.
    std::vector<double> padded;
    padded.reserve(n_frames + 2 * half_window);
    padded.insert(padded.end(), half_window, 0.0);
    padded.insert(padded.end(), mono_samples, mono_samples + n_frames);
    padded.insert(padded.end(), half_window, 0.0);

    // Slide windows.
    std::vector<double> df_values;
    if (padded.size() >= (size_t)window_size) {
        df_values.reserve((padded.size() - window_size) / step_size + 1);
        std::vector<double> window_buf(window_size, 0.0);
        for (size_t start = 0; start + window_size <= padded.size(); start += step_size) {
            std::copy(padded.begin() + start, padded.begin() + start + window_size, window_buf.begin());
            df_values.push_back(df.processTimeDomain(window_buf.data()));
        }
    }

    // Trim trailing zeros, then skip first 2 (Mixxx convention).
    size_t nz = df_values.size();
    while (nz > 0 && df_values[nz - 1] <= 0.0) --nz;
    if (nz <= 2) return 4;
    std::vector<double> df_clean(df_values.begin() + 2, df_values.begin() + nz);

    // Beat tracker.
    TempoTrackV2 tt(static_cast<float>(sample_rate), step_size);
    std::vector<int> beat_period(df_clean.size() / 128 + 1, 0);
    tt.calculateBeatPeriod(df_clean, beat_period);
    std::vector<double> beats;
    tt.calculateBeats(df_clean, beat_period, beats);

    if (beats.size() < 2) return 5;

    // Convert frame indices to seconds. Mixxx adds step_size/2 because
    // the beat is detected between two STFT frames.
    const double sr = static_cast<double>(sample_rate);
    const double half_step = step_size / 2.0;
    size_t to_emit = std::min(beats.size(), out_beats_capacity);
    for (size_t i = 0; i < to_emit; ++i) {
        double sample = beats[i] * step_size + half_step;
        out_beats_seconds[i] = sample / sr;
    }
    *out_beats_count = to_emit;

    // BPM from median inter-beat interval — robust to any single
    // misdetected beat.
    std::vector<double> intervals;
    intervals.reserve(beats.size() - 1);
    for (size_t i = 1; i < beats.size(); ++i) {
        intervals.push_back((beats[i] - beats[i - 1]) * step_size);
    }
    std::sort(intervals.begin(), intervals.end());
    double median_samples = intervals[intervals.size() / 2];
    if (median_samples <= 0.0) return 6;
    *out_bpm = 60.0 * sr / median_samples;

    return 0;
}
