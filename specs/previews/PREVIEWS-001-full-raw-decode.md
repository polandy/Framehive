# PREVIEWS-001 – Full RAW decode

- Status: implemented
- Module: previews
- Origin: F4, B4, 2.8
- Depends on: –

## Purpose

Thumbnails and the loupe come from the embedded preview, but 100% zoom, focus checking
and side-by-side comparison need the RAW decoded at full resolution (ADR 0012). This
specification fixes what a full decode returns, how it fails, and the reference decode
whose output hashes prove that the pinned decoder still decodes the reference files the
same way (ADR 0028).

## Behaviour

**The port.** The previews service defines the `PreviewDecoder` port. A full decode
takes the path of an original and a decode mode and returns a decoded image, or an
error that names the file. `adapter-libraw` implements the port with LibRaw. Every
implementation passes the port's conformance suite.

**The decoded image** has a width and a height in pixels and three 16-bit samples per
pixel, red, green and blue, in sRGB, row by row from the top left. Its size is the
decoder's visible image area. It is not rotated by the file's `Orientation` tag:
the viewer applies the orientation, as it does for the embedded preview.

**Decode modes.**

| Mode | Threads | Use |
|---|---|---|
| `Reference` | one | the version test; the output is bit-identical between runs |
| `Interactive` | all | 100% zoom and comparison; X-Trans files may differ in a few pixel values between runs (ADR 0028) |

Both modes use the same configuration otherwise: 16-bit output, sRGB, the camera's
white balance, no automatic brightening, and the decoder's default demosaicing.

**Pixel hash.** The hash of a decoded image is the BLAKE3 hash of its samples, each
as two bytes in little-endian order, in the order above. Width and height are compared
separately.

**Version test.** `adapter-libraw` decodes every RAW file in `testdata.lock` that the
pinned decoder supports, in reference mode, and compares size and pixel hash with the
values stored for the pinned decoder version on Linux x86_64. Hashes for other
platforms are added when they are measured. If a file in `testdata.lock` is missing,
the test fails and names the file and `cargo xtask testdata fetch`; it is never
skipped.

**Errors.**

| Case | Error |
|---|---|
| The file cannot be opened or read | `Io`, with the path and the cause |
| The decoder does not support the format, for example GoPro GPR with the pinned LibRaw | `UnsupportedFormat`, with the path; CORE-004 then shows the embedded preview |
| The file is truncated or its data is damaged | `Corrupt`, with the path and the decoder's message |
| Any other decoder failure, such as running out of memory | `Failed`, with the path and the decoder's message |

## Acceptance criteria

### PREVIEWS-001.1 – Decoded image layout
- **Given** a RAW file the decoder supports
- **When** it is decoded in either mode
- **Then** the decoded image has a width and a height above zero and exactly
  width × height × 3 samples

### PREVIEWS-001.2 – Reference mode is deterministic
- **Given** a RAW file the decoder supports, including an X-Trans RAF
- **When** it is decoded twice in reference mode
- **Then** both decoded images have the same size and the same samples

### PREVIEWS-001.3 – Interactive mode has the reference size
- **Given** a RAW file the decoder supports
- **When** it is decoded in interactive mode and in reference mode
- **Then** both decoded images have the same width and height

### PREVIEWS-001.4 – Reference hashes
- **Given** the RAW files in `testdata.lock` that the pinned decoder supports
- **When** each is decoded in reference mode on Linux x86_64
- **Then** its width, height and pixel hash match the values stored for the pinned
  decoder version

### PREVIEWS-001.5 – Pinned decoder version
- **Given** the decoder built by the Nix flake
- **When** its version is read
- **Then** it is the version the reference hashes were stored for, LibRaw 0.22.1

### PREVIEWS-001.6 – Unsupported format
- **Given** a file in a format the decoder does not support, such as GoPro GPR
- **When** it is decoded
- **Then** the decode fails with `UnsupportedFormat` naming the file

### PREVIEWS-001.7 – Corrupt file
- **Given** a RAW file truncated to half its length
- **When** it is decoded
- **Then** the decode fails with `Corrupt` naming the file

### PREVIEWS-001.8 – Missing file
- **Given** a path where no file exists
- **When** it is decoded
- **Then** the decode fails with `Io` naming the path, and the cause is "not found"

## Out of scope

- Thumbnails and the loupe preview from embedded JPEGs: CORE-001, CORE-003, CORE-004.
- The preview cache and the decode job: specified with the previews cache.
- HEIF, video and Immich thumbnails: later preview decoders.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-03 | Created from the phase 0 LibRaw prototype (ADR 0028) |
| 2026-11-03 | Implemented by `adapter-libraw` |
| 2026-11-07 | JPEG XL DNG dropped as an `UnsupportedFormat` example: no CC0 sample tests it, and the pinned LibRaw may report a data error instead (ADR 0028) |
| 2026-11-09 | Reference hash of the owner's Canon EOS 350D Lightroom DNG (`dng/canon-eos-350d/`) added |
