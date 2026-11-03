<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# adapter-libraw

Layer: `adapter`, port `previews` (architecture §3)

## Purpose

Implements the `PreviewDecoder` port of `previews` with LibRaw (ADR 0012): the full
RAW decode to 16-bit sRGB for 100% zoom and comparison (PREVIEWS-001). It binds
LibRaw's thread-safe C API, `libraw_r`, and is the only crate with `unsafe` code for
it. Embedded previews, the preview cache and orientation are left to other crates.

## Public API

| Item | Meaning |
|---|---|
| `LibRawDecoder` | `previews::PreviewDecoder` backed by LibRaw; the reference mode decodes with one OpenMP thread (ADR 0028 §3) |

## Conformance

`tests/conformance.rs` runs `previews::testing::check_preview_decoder` with an X-T5
lossy RAF, a GoPro GPR and a truncated RAF from `testdata.lock`.
`tests/reference_decode.rs` is the version test: it checks the pinned LibRaw version
and the size and pixel hash of every supported RAW in `testdata.lock`. Both need
`cargo xtask testdata fetch` and fail without the files.

## Dependencies

- `previews`
- LibRaw 0.22.1 with zlib and libjpeg, and LLVM's OpenMP on macOS, a native library from the Nix flake
  (ADR 0028 §2), and its OpenMP runtime (libgomp, libomp on macOS).
- `src/ffi.rs` declares the C functions by hand; `src/shim.c` sets the two decode
  parameters the C API has no setter for. `build.rs` compiles the shim with `$CC` and
  `ar` (ADR 0028 §5).
- Development only: `blake3`, for the pixel hashes of the version test.
