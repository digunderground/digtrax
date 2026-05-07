// SPDX-License-Identifier: GPL-3.0-or-later
// Force-included before every Rubber Band translation unit (see
// build.rs).
//
// Why this exists: Rubber Band's `common/mathmisc.h`, `Scavenger.h`,
// and `StretchCalculator.h` use bare `size_t` (and a few `int64_t`)
// without `std::` prefix or any include of `<cstddef>`. That worked
// historically because most C standard headers leak `size_t` into
// the global namespace, but modern libc++ in Xcode 16+ keeps it in
// `std::` only — leading to "unknown type name 'size_t'" errors.
//
// Rather than patching ~20 lines across 4 files, we force-include
// this header which pulls in `<cstddef>` and `<cstdint>` and aliases
// the std types into the global namespace, exactly as the legacy
// expectation assumes.

#pragma once

#include <cstddef>
#include <cstdint>

using std::size_t;
using std::int8_t;
using std::int16_t;
using std::int32_t;
using std::int64_t;
using std::uint8_t;
using std::uint16_t;
using std::uint32_t;
using std::uint64_t;
