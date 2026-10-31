<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# metadata

Layer: `service` (architecture §2, §3)

## Purpose

The metadata service reads metadata from photo files. It reads the capture time
from EXIF with its own parser (CORE-005, ADR 0021).

## Public API

| Item | Meaning |
|---|---|
| `capture_time_from_bytes` | The capture time of a file's content in memory |
| `NoCaptureTime` | Why a file has no capture time |
| `testing::Tiff`, `testing::ByteOrder`, `testing::jpeg`, `testing::raf` | Builders for test files with EXIF text tags |

## Dependencies

- `domain`
- `thiserror` (ADR 0016)
