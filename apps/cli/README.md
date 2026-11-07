<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# framehive-cli

Layer: `app` (architecture §3). Binary: `framehive`.

## Purpose

The command line of Framehive: `framehive import` (CLI-001, CLI-004, CLI-010, CLI-011,
CLI-013), `framehive cameras` (CLI-013), `framehive import-log` (CLI-009),
`framehive config init` (CLI-010) and, without a subcommand, the guided import
(CLI-002, CLI-003, ADR 0024). It parses the arguments
with `clap` (builder API, ADR 0020 §6) and is one of the two places, with
`framehive-gui`, that know the concrete adapters and wire them to the ports.

## Usage

```
framehive
framehive import <source>... <destination> [--backup <folder>]...
                 [--video-destination <folder>] [--video-backup <folder>]...
                 [--template <template>] [--job-code <text>]
                 [--on-conflict add_suffix|skip|fail] [--dry-run]
framehive import --destination <folder> <source>... [other options]
framehive import <source>... <destination> --profile <name> [other options]
framehive import camera:[<model>|<port>] <destination> [other options]
framehive cameras
framehive import-log [<run>|last] [--format text|json|csv]
framehive config init
framehive --help
framehive --version
```

## Sections

- [The import command](docs/import-command.md): `framehive import`, from the arguments
  to the jobs of its sources (CLI-001, CLI-004, CLI-005, CLI-011, CLI-012, CLI-015, CLI-016)
- [Import profiles](docs/import-profiles.md): `--profile` and `framehive config init`
  (CLI-010)
- [Cameras](docs/cameras.md): `camera:` sources and `framehive cameras` (CLI-013)
- [Guided import](docs/guided-import.md): `framehive` without a subcommand (CLI-002,
  CLI-003, CLI-014)
- [Import log](docs/import-log.md): `framehive import-log` (CLI-009)
- [Report and exit codes](docs/report.md): the report lines and the exit codes
- [Tests](docs/tests.md): how the tests run the binary

## Modules

| Module | Meaning |
|---|---|
| `arguments` | The `clap` command line and the parsed `Invocation` |
| `paths` | Source and destination resolution, the inside check, creating the destination |
| `import_command` | Wires the adapters to the import job of each source (`run.rs`, tested in-crate with `MemoryFreeSpace` and `MemoryCameras`), follows the jobs on threads of their own (`sources.rs`) and maps the results to the exit code |
| `cameras` | `camera:` sources and `framehive cameras` (CLI-013): naming, finding and opening a camera |
| `report` | The `ProgressSink` that prints the report lines and the summary |
| `profiles` | The configuration folder, profile names, and reading the profile of `--profile` (CLI-010) |
| `config_init` | `framehive config init`: the example profile (CLI-010) |
| `import_log` | `framehive import-log` (CLI-009): reading the history, and the `text`, `json` and `csv` writers |
| `format_readiness` | The format readiness lines (CLI-007), shared by `framehive import` and the guided import |
| `guided` | The guided import (CLI-002, CLI-003, CLI-014): `dialog` (prompts, waiting, remembered answers), `source` (a card or an opened camera), `device` (opening and reading, eject or release), `card` (summary, import, report), `summary` (plan summary, blockers), `template` (template step, job code), `progress` (progress line, stop on a pulled card), `format` (sizes, rate, time left), `state` (state file), `events` (input events) |

## Dependencies

- `adapter-blake3`, `adapter-clock`, `adapter-fs`, `adapter-gphoto2`, `adapter-mounts`,
  `adapter-sequence-counters`, `adapter-sqlite`, `adapter-yaml`, `config`, `devices`,
  `domain`, `files`, `history`, `import`, `jobs`, `naming`

- `clap` (ADR 0020 §6): `default-features = false`, features `std`, `help`, `usage`,
  `error-context`.
