<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# domain

Layer: `domain` (architecture §3)

## Purpose

Pure business types of Framehive: assets, ratings, tags, regions and the other terms
of the glossary (architecture §10). The crate has no I/O. Time, IDs, randomness,
files and devices are reached only through ports defined by the layers above.

## Public API

| Item | Meaning |
|---|---|
| `AssetFileRole` | Role of one file of an asset: `Raw`, `Rendered`, `Video`, `Sidecar` |
| `Rating`, `RatingError` | Star rating of an asset, 0–5 |

## Dependencies

- `thiserror` (ADR 0016)
