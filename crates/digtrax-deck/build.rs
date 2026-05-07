// SPDX-License-Identifier: GPL-3.0-or-later
//
// Compiles the vendored QM-DSP source + our C shim into a static lib.
// QM-DSP and the kissfft submodule are GPL-2-or-later (see
// vendor/qm-dsp/COPYING + vendor/qm-dsp/ext/kissfft/). Linking them
// yields an aggregate work that DigTrax distributes under GPL-3 (see
// LICENSE at the project root + CREDITS.md for attribution).

fn main() {
    let qm_root = "vendor/qm-dsp";

    // C++ pieces — QM-DSP itself.
    let mut cpp = cc::Build::new();
    cpp.cpp(true)
        .std("c++17")
        .include(qm_root)
        .include("vendor")
        .flag_if_supported("-Wno-deprecated")
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-variable")
        .flag_if_supported("-Wno-sign-compare")
        // MSVC's <cmath> doesn't define M_PI by default — only when
        // _USE_MATH_DEFINES is set before <math.h> is included.
        // QM-DSP uses M_PI throughout; without this the Windows build
        // fails with "undeclared identifier 'M_PI'". POSIX toolchains
        // ignore this define so it's safe to set unconditionally.
        .define("_USE_MATH_DEFINES", None)
        // Force kissfft to use double precision so it matches QM-DSP's
        // double-precision pipeline. Default is float; QM-DSP wants double.
        .define("kiss_fft_scalar", "double")
        .file(format!("{}/maths/MathUtilities.cpp", qm_root))
        .file(format!("{}/dsp/onsets/DetectionFunction.cpp", qm_root))
        .file(format!("{}/dsp/tempotracking/TempoTrackV2.cpp", qm_root))
        .file(format!("{}/dsp/phasevocoder/PhaseVocoder.cpp", qm_root))
        .file(format!("{}/dsp/transforms/FFT.cpp", qm_root))
        .file("vendor/qm_shim.cc")
        .compile("qmdsp");

    // C piece — kissfft. Needs the same kiss_fft_scalar=double define so
    // its function signatures match what FFT.cpp links against.
    let mut c = cc::Build::new();
    c.std("c11")
        .include(qm_root)
        .include(format!("{}/ext/kissfft", qm_root))
        .flag_if_supported("-Wno-deprecated")
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-variable")
        // Same MSVC M_PI fix as the C++ build above.
        .define("_USE_MATH_DEFINES", None)
        .define("kiss_fft_scalar", "double")
        .file(format!("{}/ext/kissfft/kiss_fft.c", qm_root))
        .file(format!("{}/ext/kissfft/tools/kiss_fftr.c", qm_root))
        .compile("kissfft");

    // Re-run if any vendored or shim file changes.
    println!("cargo:rerun-if-changed=vendor/qm_shim.cc");
    println!("cargo:rerun-if-changed=vendor/qm-dsp");
}
