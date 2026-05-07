// SPDX-License-Identifier: GPL-3.0-or-later
//
// Compiles the vendored native DSP libraries into static libs:
//   1. QM-DSP (beat tracking) + kissfft  — GPL-2-or-later
//   2. Rubber Band Library (time-stretch / key lock) — GPL-2-or-later
// Both license-compatible with DigTrax's GPL-3. See COPYING files
// in each vendor directory and CREDITS.md for attribution.

fn main() {
    build_qmdsp();
    build_rubberband();
}

fn build_qmdsp() {
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

fn build_rubberband() {
    // Rubber Band Library — used for the per-deck Key Lock
    // (pitch-preserving time-stretch). We compile only the R2 ("Faster")
    // and R3 ("Finer") engines plus their shared `common/` utilities; we
    // skip JNI bindings, LADSPA/LV2/Vamp plugin hosts, and CLI tools
    // since those aren't needed by an in-process FFI consumer.
    //
    // Defines:
    //   USE_BUILTIN_FFT  — Rubber Band's own FFT impl (no external dep
    //                      like FFTW / kissfft / vDSP / IPP / Sleef).
    //                      Slightly slower than vDSP on macOS but works
    //                      on every platform with no extra source.
    //   USE_BQRESAMPLER  — Rubber Band's bundled BQResampler (no
    //                      libsamplerate / speex / IPP dep).
    //   USE_PTHREADS     — POSIX thread API on Unix; Windows uses the
    //                      Win32 thread fallback in Thread.cpp.
    //   _USE_MATH_DEFINES — MSVC <cmath> needs this for M_PI (same fix
    //                      we applied for QM-DSP).
    let rb_root = "vendor/rubberband";
    let mut rb = cc::Build::new();
    rb.cpp(true)
        .std("c++17")
        .include(rb_root)              // rubberband/RubberBandStretcher.h
        .include(format!("{}/src", rb_root)) // common/Allocators.h, etc
        .define("_USE_MATH_DEFINES", None)
        .define("USE_BUILTIN_FFT", None)
        .define("USE_BQRESAMPLER", None)
        .flag_if_supported("-Wno-deprecated")
        .flag_if_supported("-Wno-deprecated-declarations")
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-variable")
        .flag_if_supported("-Wno-unused-but-set-variable")
        .flag_if_supported("-Wno-unused-private-field")
        .flag_if_supported("-Wno-sign-compare")
        .flag_if_supported("-Wno-reorder")
        .flag_if_supported("-Wno-shift-negative-value");

    // Threading backend — only Unix uses pthreads; Windows takes the
    // Win32 branch automatically based on `#ifdef _WIN32` in Thread.cpp.
    if !cfg!(target_os = "windows") {
        rb.define("USE_PTHREADS", None);
    }

    // sincosf / sincos exist on glibc but NOT on macOS / MSVC.
    // RubberBand's VectorOpsComplex.h gates the call with `__GNUC__`,
    // which clang on macOS also defines, so we'd hit "use of undeclared
    // identifier 'sincosf'". Setting LACK_SINCOS forces the cosf/sinf
    // fallback branch, which is correct on every non-glibc platform.
    if cfg!(target_os = "macos")
        || cfg!(target_os = "ios")
        || cfg!(target_env = "msvc")
        || cfg!(target_os = "windows")
    {
        rb.define("LACK_SINCOS", None);
    }

    // Force-include a compat header before every TU. Modern libc++
    // (Xcode 16+) and recent stdlibs keep `size_t` strictly in `std::`,
    // breaking RubberBand's bare `size_t` usage. The compat header
    // pulls in <cstddef>/<cstdint> and aliases the std types globally.
    let compat = "vendor/rubberband_compat.h";
    if cfg!(target_env = "msvc") {
        rb.flag(format!("/FI{}", compat));
    } else {
        rb.flag("-include");
        rb.flag(compat);
    }

    // Source files. The list mirrors meson.build's `rubberband_src`
    // target with `cmdline=disabled,jni=disabled,ladspa=disabled,
    // lv2=disabled,vamp=disabled` — i.e. just the library itself.
    let rb_files = [
        // Common (used by both R2 and R3)
        "src/common/Allocators.cpp",
        "src/common/BQResampler.cpp",
        "src/common/FFT.cpp",
        "src/common/Log.cpp",
        "src/common/Profiler.cpp",
        "src/common/Resampler.cpp",
        "src/common/StretchCalculator.cpp",
        "src/common/Thread.cpp",
        "src/common/VectorOpsComplex.cpp",
        "src/common/mathmisc.cpp",
        "src/common/sysutils.cpp",
        // R2 ("Faster" — Mixxx's default keylock engine)
        "src/faster/AudioCurveCalculator.cpp",
        "src/faster/CompoundAudioCurve.cpp",
        "src/faster/HighFrequencyAudioCurve.cpp",
        "src/faster/PercussiveAudioCurve.cpp",
        "src/faster/R2Stretcher.cpp",
        "src/faster/SilentAudioCurve.cpp",
        "src/faster/StretcherChannelData.cpp",
        "src/faster/StretcherProcess.cpp",
        // R3 ("Finer") — referenced unconditionally by RubberBandStretcher.cpp
        "src/finer/R3Stretcher.cpp",
        "src/finer/R3LiveShifter.cpp",
        // Top-level facades + C API
        "src/RubberBandStretcher.cpp",
        "src/RubberBandLiveShifter.cpp",
        "src/rubberband-c.cpp",
    ];
    for f in rb_files {
        rb.file(format!("{}/{}", rb_root, f));
    }
    rb.compile("rubberband");

    println!("cargo:rerun-if-changed=vendor/rubberband");
}
