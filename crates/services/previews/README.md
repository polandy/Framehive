<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# previews

Layer: `service` (architecture §2, §3)

## Purpose

The previews service holds the port through which Framehive decodes a RAW at full
resolution, for 100% zoom and comparison (PREVIEWS-001, ADR 0012).

Boundaries:

- **Port only.** The service never decodes; `adapter-libraw` implements
  `PreviewDecoder` with LibRaw.
- **Decoded image.** Three 16-bit sRGB samples per pixel, row by row from the top left,
  not rotated by the `Orientation` tag. `DecodedImage::new` rejects samples that do not
  fit the size.
- **Two modes.** `Reference` decodes with one thread and is bit-identical between runs;
  the version test hashes it. `Interactive` uses all threads (ADR 0028 §3).
- **Errors name the file:** `Io`, `UnsupportedFormat` (CORE-004 then shows the
  embedded preview), `Corrupt` and `Failed`.
- **Not covered here:** thumbnails from embedded JPEGs, the preview cache and the
  decode job.

## Public API

| Item | Meaning |
|---|---|
| `PreviewDecoder` | Port that reports the decoder version and decodes a RAW at full resolution |
| `DecodeMode` | `Reference` (one thread, deterministic) or `Interactive` (all threads) |
| `DecodedImage` | Width, height and RGB samples of a full decode; `sample_bytes` is the input of the pixel hash |
| `DecodeError` | `Io`, `UnsupportedFormat`, `Corrupt` or `Failed`, each with the path (PREVIEWS-001.6–8) |
| `ImageLayoutError` | The samples given for a `DecodedImage` do not fit its size |
| `testing::MemoryPreviewDecoder` | `PreviewDecoder` in memory, with images, unsupported and corrupt files per path |
| `testing::check_preview_decoder`, `testing::PreviewDecoderUnderTest` | Conformance suite of the `PreviewDecoder` port |

## Dependencies

- `thiserror` (ADR 0016)
