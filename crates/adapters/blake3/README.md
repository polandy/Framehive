<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# adapter-blake3

Layer: `adapter`, port `files` (architecture §3)

## Purpose

Implements the `Hasher` port of `files` with BLAKE3, the checksum of Framehive
(architecture §8, ADR 0020 §5). The verified copy hashes each file with it while it
copies and again when it reads the copy back. The `blake3` crate is used here and
nowhere else; no other crate sees one of its types.

## Public API

| Item | Meaning |
|---|---|
| `Blake3Hasher` | `files::Hasher` whose checksums carry the algorithm `blake3` (`domain::Checksum::BLAKE3`) and the 32-byte BLAKE3 digest |

## Conformance

`tests/conformance.rs` runs `files::testing::check_hasher` with reference checksums
from the official BLAKE3 test vectors.

## Dependencies

- `domain`, `files`
- `blake3` (ADR 0020 §5): `default-features = false`, feature `std`. Its SIMD paths
  contain assembly and C, built by its build script; the feature `pure` drops them if
  they ever break the reproducible build (ADR 0007).
