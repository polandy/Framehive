<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# framehive-cli

Layer: `app` (architecture §3). Binary: `framehive`.

## Purpose

The command line of Framehive: `framehive import` (CLI-001, CLI-004, CLI-010, CLI-011,
CLI-013), `framehive cameras` (CLI-013), `framehive import-log` (CLI-009),
`framehive config init` (CLI-020), `framehive library keep-raw` and
`framehive library remove-raws` (CLI-018) and, without a subcommand, the guided import
(CLI-002, CLI-003, ADR 0024). It parses the arguments
with `clap` (builder API, ADR 0020 §6) and is one of the two places, with
`framehive-gui`, that know the concrete adapters and wire them to the ports.

## Usage

```
framehive
framehive import <source>... <destination> [--backup <folder>]...
                 [--video-destination <folder>] [--video-backup <folder>]...
                 [--template <template>] [--video-template <template>]
                 [--job-code <text>]
                 [--on-conflict add_suffix|skip|fail] [--dry-run]
framehive import --destination <folder> <source>... [other options]
framehive import <source>... <destination> --profile <name> [other options]
framehive import camera:[<model>|<port>] <destination> [other options]
framehive cameras
framehive import-log [<run>|last] [--format text|json|csv]
framehive config init
framehive library keep-raw <file>... [--off] [--dry-run]
framehive library remove-raws <folder> [--dry-run] [--yes]
framehive --help
framehive --version
```

`-v` or `--verbose`, before or after the subcommand, writes diagnostic lines to
standard error; twice or three times for more detail (CLI-021, ADR 0046).
`framehive --help` ends with examples, the folders and the exit codes (CLI-023).

Flag conventions for every command (CLI-023):

- `--dry-run` on each command that copies, marks or moves photos and videos: it checks
  and prints as a real run would, and changes nothing.
- `--yes` on each command outside the guided import that asks before it changes files,
  for scripts; without it and without an answer the command stops with exit code 2.
- Long, full-word flags; `-h`, `-v` and `-V` are the only short ones.
- Exit codes: 0 done, 1 ran but a file failed, 2 did not start.

## Sections

- [The import command](docs/import-command.md): `framehive import`, from the arguments
  to the jobs of its sources (CLI-001, CLI-004, CLI-005, CLI-011, CLI-012, CLI-015, CLI-016)
- [Import profiles](docs/import-profiles.md): `--profile` and `framehive config init`
  (CLI-010, CLI-020)
- [Cameras](docs/cameras.md): `camera:` sources and `framehive cameras` (CLI-013)
- [Guided import](docs/guided-import.md): `framehive` without a subcommand (CLI-002,
  CLI-003, CLI-014)
- [Import log](docs/import-log.md): `framehive import-log` (CLI-009)
- [Library commands](docs/library.md): `framehive library keep-raw` and
  `framehive library remove-raws` (CLI-018)
- [Report and exit codes](docs/report.md): the report lines, the sidecar lines
  (CLI-017) and the exit codes
- [Tests](docs/tests.md): how the tests run the binary

## Modules

| Module | Meaning |
|---|---|
| `arguments` | The `clap` command line and the parsed `Invocation`; `framehive library` in `library.rs` |
| `fault_injection` | Only with the feature `fault-injection` (ADR 0048): `FaultyFileSystem`, which wraps the file system of the import and the guided import; without a fault it passes every call through. Its tests are in `tests/fault_injection.rs`, run by the CI step `fault injection` |
| `diagnostics` | The `tracing` subscriber on standard error, by `--verbose` (CLI-021, ADR 0046) |
| `paths` | Source and destination resolution, the inside check, creating the destination |
| `import_command` | Wires the adapters to the import job of each source (`run.rs`, tested in-crate with `MemoryFreeSpace` and `MemoryCameras`), follows the jobs on threads of their own (`sources.rs`) and maps the results to the exit code |
| `cameras` | `camera:` sources and `framehive cameras` (CLI-013): naming, finding and opening a camera |
| `report` | The `ProgressSink` that prints the report lines and the summary |
| `profiles` | The configuration folder, profile names, and reading the profile of `--profile` (CLI-010) |
| `config_init` | `framehive config init`: the example profile (CLI-020) |
| `import_log` | `framehive import-log` (CLI-009): reading the history, and the `text`, `json` and `csv` writers |
| `library_command` | `framehive library keep-raw` and `remove-raws` (CLI-018): the adapters and the trash of the system (`mod.rs`), marking (`keep_raw.rs`) and the summary, question and run (`remove_raws.rs`) |
| `format_readiness` | The format readiness lines (CLI-007), shared by `framehive import` and the guided import |
| `guided` | The guided import (CLI-002, CLI-003, CLI-014): `dialog` (prompts, waiting, remembered answers), `source` (a card or an opened camera), `device` (opening and reading, eject or release), `card` (summary, import, report), `summary` (plan summary, blockers), `template` (template step, job code), `progress` (progress line, stop on a pulled card), `format` (sizes, rate, time left), `state` (state file), `events` (input events) |

## Dependencies

- `adapter-blake3`, `adapter-clock`, `adapter-free-space`, `adapter-freedesktop-trash`,
  `adapter-fs`, `adapter-gphoto2`, `adapter-macos-trash`, `adapter-mounts`,
  `adapter-sequence-counters`, `adapter-sqlite`, `adapter-xmp`, `adapter-yaml`, `config`,
  `devices`, `domain`, `files`, `history`, `import`, `jobs`, `library`, `metadata`,
  `naming`

- `clap` (ADR 0020 §6): `default-features = false`, features `std`, `help`, `usage`,
  `error-context`.
- `tracing`, and `tracing-subscriber` with only `fmt` (ADR 0046): the diagnostic
  output of `--verbose`.
