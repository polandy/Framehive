<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# domain

Layer: `domain` (architecture §3)

## Purpose

Pure business types of Framehive: assets, ratings, tags, regions and the other terms
of the glossary (architecture §10). The crate has no I/O. Time, IDs, randomness,
files and devices are reached only through ports. The `Clock` port is defined here;
the others are defined by the layers above. Times are own std-only types, and no
time library is visible here (ADR 0022).

## Public API

| Item | Meaning |
|---|---|
| `AssetFileRole` | Role of one file of an asset: `Raw`, `Rendered`, `Video`, `Sidecar` |
| `Rating`, `RatingError` | Star rating of an asset, 0–5 |
| `Timestamp` | An instant in UTC, to the nanosecond, since the Unix epoch |
| `UtcOffset` | Offset of local time from UTC, in seconds, below a day |
| `TimeError` | Why values are not a valid `Timestamp` or `UtcOffset` |
| `Checksum`, `ChecksumError` | Hash of a file's content with its algorithm, written `<algorithm>:<hex>` (architecture §8) |
| `Clock` | Port for the current `Timestamp` and the local `UtcOffset` at a `Timestamp` (ADR 0020, ADR 0022) |
| `testing::ManualClock` | `Clock` for tests that shows the time it is set to |

## Dependencies

- `thiserror` (ADR 0016)
