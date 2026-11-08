# 0028 – LibRaw build and reference decode, from the phase 0 prototype

- Status: accepted
- Date: 2026-11-03
- Amends: ADR 0012, decisions 3 and 5, in how LibRaw is built and how the reference
  decode is configured

## Context

ADR 0012 chose LibRaw for RAW decoding and asked phase 0 to validate it with a
prototype: X-T5 lossy compressed RAF must decode, otherwise a new ADR supersedes 0012.
The prototype also checks GoPro GPR (VC-5) and JPEG XL compressed DNG.

The prototype ran LibRaw's own sample programs (`raw-identify`, `dcraw_emu -w -6 -o 1`)
from LibRaw 0.22.1 in the pinned nixpkgs against the samples in `testdata.lock`, all
from raw.pixls.us under CC0:

| Sample | Result with nixpkgs `libraw` |
|---|---|
| X-T5 RAF, lossless, lossy and uncompressed (`RAFCompression`), and a Sports Finder crop | decoded; a 100% crop of the lossy file shows no artefacts |
| Canon EOS 7D CR2: RAW, M-RAW, S-RAW | decoded |
| Adobe DNG Converter DNG, lossless JPEG | decoded |
| Adobe DNG Converter DNG, lossy JPEG | "Unsupported file format" |
| HDRMerge DNG, 16-bit floating point, deflate | "Corrupted data or unexpected EOF" |
| GoPro HERO11 GPR (VC-5) | "Unsupported file format" |
| DNG with JPEG XL (DNG 1.7) | not tested, no CC0 sample; the library contains only `jxl_dng_load_raw_placeholder` |

Two findings follow from it:

1. **The nixpkgs build links neither zlib nor libjpeg** (only lcms2 and libgomp). LibRaw
   needs zlib for deflate DNG and libjpeg for lossy DNG. Built with both, the same
   LibRaw version decodes both samples, and the images look right.
2. **The X-Trans demosaicing is not deterministic with OpenMP.** Every run of the same
   RAF gives a different output hash (about 192,000 differing bytes for the lossy
   sample). With one OpenMP thread the output is bit-identical between runs. Bayer
   files (7D) and `-q 0` (no demosaicing) are deterministic with all threads. One
   thread is about six times slower: 18.5 s instead of 3.0 s for one X-T5 file on
   16 threads.

## Decision

1. ADR 0012 stands: LibRaw decodes X-T5 lossy compressed RAF in the pinned version.
2. **The flake builds LibRaw from nixpkgs `libraw` with zlib and libjpeg added, and
   on macOS with LLVM's OpenMP runtime.** nixpkgs builds it on macOS without OpenMP,
   because Apple's clang has no OpenMP runtime; with it, both platforms decode with
   the threads of decision 3 and one code path. It adds neither the Adobe DNG SDK nor
   the GoPro GPR SDK, so GPR and JPEG XL DNG are not decoded; CORE-004 shows the
   embedded preview for them.
3. **The reference decode uses one OpenMP thread.** It is part of the fixed decode
   configuration of the version test (ADR 0012, decision 3), so stored hashes stay
   valid. Interactive decoding uses all threads, because a few differing pixel values
   do not matter on screen.
4. The non-deterministic X-Trans demosaicing is reported to LibRaw upstream. If a fix
   is released, a later ADR may drop the one-thread rule.
5. **`adapter-libraw` binds LibRaw's C API (`libraw_r`) by hand, with a small C shim.**
   The FFI declarations are written out in `src/ffi.rs`, without bindgen or a
   pkg-config crate; the Nix shell provides the include and library paths. The C API
   has no setters for `use_camera_wb` and `user_flip`, so `src/shim.c` sets these two
   fields of `imgdata.params` by name, which keeps the struct layout out of Rust.
   `build.rs` compiles the shim with `$CC` and `ar` through `std::process`, without a
   build dependency. The adapter reads the file itself and passes it to
   `libraw_open_buffer`, so a missing file is an I/O error with its cause. The
   reference decode sets one thread with `omp_set_num_threads` for the decoding thread
   and restores the previous value afterwards.

## Alternatives

- **Compare the reference decode with a tolerance instead of a hash:** it allows
  multi-threaded decoding in the test, but needs our own image comparison and is less
  strict. A hash with one thread is simpler and catches every change.
- **Disable OpenMP in the LibRaw build:** deterministic everywhere, but X-Trans decoding
  becomes about six times slower for every user.
- **Build LibRaw with the Adobe DNG SDK and the GPR SDK:** decodes GPR and JPEG XL DNG,
  but adds two large native libraries for formats that have an embedded preview. It
  can be revisited when the owner's own files need it.

## Consequences

- The flake carries a small override of `libraw`; a nixpkgs update can change the
  LibRaw version, which the version test then shows.
- The version test takes about 20 s per X-Trans file. It runs with the test data that
  `cargo xtask testdata fetch` provides. The conformance suite needs two reference
  decodes of an X-Trans RAF, so the LibRaw conformance test shares one of them with
  the version test of that RAF and runs it beside the other, instead of decoding the
  RAF three times one after another.
- `testdata.lock` pins the 7D, DNG and GPR samples used here, until `framehive-testdata`
  provides the reference sets (ADR 0017).
