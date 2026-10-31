<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# metadata

Layer: `service` (architecture §2, §3)

## Purpose

The metadata service reads metadata from photo files. It reads the capture time
from EXIF with its own parser (CORE-005, ADR 0021). It reads only the few
kilobytes that lead to the EXIF, never a whole RAW file.

## Public API

| Item | Meaning |
|---|---|
| `read_capture_time` | The capture time of a file, read through the `FileSystem` port; formats not in the CORE-005 table are not opened |
| `CaptureTimeError` | Why `read_capture_time` gives no capture time: a `NoCaptureTime` reason or an I/O error with the path |
| `capture_time_from_bytes` | The capture time of a file's content in memory |
| `NoCaptureTime` | Why a file has no capture time |
| `testing::Tiff`, `testing::ByteOrder`, `testing::jpeg`, `testing::raf` | Builders for test files with EXIF text tags |

## Dependencies

- `domain`
- `files` (the `FileSystem` port)
- `thiserror` (ADR 0016)
