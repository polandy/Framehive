<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# framehive-cli

Layer: `app` (architecture §3). Binary: `framehive`.

## Purpose

The command line of Framehive (CLI-001). It parses the arguments with `clap` (builder
API, ADR 0020 §6) and is one of the two places, with `framehive-gui`, that know the
concrete adapters and wire them to the ports.

## Usage

```
framehive import <source> <destination> [--template <template>]
                 [--on-conflict add_suffix|skip|fail] [--dry-run]
framehive --help
framehive --version
```

## Dependencies

- `clap` (ADR 0020 §6): `default-features = false`, features `std`, `help`, `usage`,
  `error-context`.
