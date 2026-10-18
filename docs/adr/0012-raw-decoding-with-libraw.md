# 0012 – RAW decoding with LibRaw

- Status: proposed
- Date: 2026-10-17

## Context

The Fujifilm X-T2 and X-T5 are reference cameras (requirements 2.8). Their RAF files
use an X-Trans sensor. It has a 6×6 colour filter pattern instead of a 2×2 Bayer
pattern, and needs its own demosaicing. The X-T5 also writes lossy compressed RAF.

The embedded JPEG in an RAF is enough for thumbnails and the normal loupe view (F4). It
is not full resolution, so 100% zoom, focus checking while culling and side-by-side
comparison need a real RAW decode in the MVP.

A RAW decoder is a large, domain-deep problem. It covers hundreds of camera formats,
compression schemes, colour matrices and constant updates for new cameras. Besides
the Fujifilm cameras, the Canon EOS 7D (CR2, including the YCbCr-based M-RAW and S-RAW)
and DNG (mosaic, linear, lossy, drone and GoPro GPR files) are reference formats
(requirements 2.8). One decoder should cover all of them. ADR 0009
therefore allows a library, but a native one needs its own ADR.

## Decision

1. **LibRaw** decodes RAW files. It is wrapped in exactly one adapter crate,
   `adapter-libraw`, that implements the `PreviewDecoder` port. The FFI is our own thin
   binding, and every `unsafe` block carries a `// SAFETY:` comment.
2. **Embedded previews and metadata are read by our own code**, not by LibRaw. This
   covers the RAF header, the embedded JPEG, EXIF, the Fujifilm and Canon maker notes,
   and the TIFF structure shared by CR2 and DNG (ADR 0009, CORE-001, CORE-003,
   CORE-004). Thumbnails and the grid never need the native library.
3. **A minimum LibRaw version** is pinned in the Nix flake and checked by a test. The
   test decodes the X-T2/X-T5, Canon 7D and DNG reference sets and compares the
   output against stored hashes from a fixed decode configuration.
4. On macOS, LibRaw is linked as a dynamic library and bundled in the app.
5. Phase 0 validates the decision with a prototype. If LibRaw cannot decode X-T5 lossy
   compressed RAF in the pinned version, this ADR is revisited before it is accepted.
   The prototype also checks GoPro GPR (VC-5) and JPEG XL compressed DNG. Missing support
   there does not block the decision; CORE-004 defines a preview fallback.

## Alternatives

- **rawler (dnglab), pure Rust:** no native dependency, supports X-Trans. However, it
  has a small maintainer base and fewer users, so it does not meet the "established"
  criteria of ADR 0009. We re-evaluate it once it matures; the port makes a switch
  local to one adapter.
- **rawspeed (darktable):** mature, but its C++ API is not stable and it is not meant
  for use outside darktable. It also needs its own demosaicing.
- **Embedded JPEG only:** no dependency, but no 100% zoom and no comparison at full
  resolution. That is not acceptable for culling (B4, N10).
- **Calling `darktable-cli` as a process:** too slow for interactive zoom, and adds a
  heavy runtime dependency.

## Consequences

- One native dependency more. It is mitigated by the Nix pin, the version test and the
  single adapter crate.
- LibRaw is dual-licensed (LGPL-2.1 or CDDL-1.0). Dynamic linking satisfies LGPL. The
  choice of project licence must stay compatible (requirements, open question 1).
- New cameras need a LibRaw update. The reference-set test shows whether an update
  breaks existing files.
