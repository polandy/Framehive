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

`framehive import` parses the name template, resolves the source without symbolic
links, resolves the longest existing part of the destination and appends the missing
folders (rejecting `..` among them), and rejects a destination inside the source.
Unless it is a dry run, it then creates the missing folders of the destination and
runs `import::ImportRun` with `adapter-fs`, `adapter-blake3` and `adapter-clock`, in
compatibility mode.

## Report and exit codes

The report texts live here, not in the import module. Standard output gets one line
per file as soon as it is done (`copied`, `would copy`, `already imported`,
`duplicate`, `skipped`, `failed`), the files and folders the scan left out
(`left out`), and a summary line; in a dry run `copied` reads `would copy`, in the
summary as well. Errors that stop the command go to standard error as
`error: <message>`; there is no log output yet.

| Code | Meaning |
|---|---|
| 0 | Every file was copied, already imported, a duplicate, skipped or left out |
| 1 | A file failed, or a folder could not be read (`ImportSummary::has_failures`) |
| 2 | The import did not start: invalid arguments or template, missing source, destination inside the source or not creatable |

## Modules

| Module | Meaning |
|---|---|
| `arguments` | The `clap` command line and the parsed `Invocation` |
| `paths` | Source and destination resolution, the inside check, creating the destination |
| `import_command` | Wires the adapters to the import run and maps the result to the exit code |
| `report` | The `ProgressSink` that prints the report lines and the summary |

## Tests

`tests/` runs the real binary (`CARGO_BIN_EXE_framehive`) in a temporary folder with
`TZ=UTC`, with test files from `metadata::testing`. Unreadable files and folders use
mode 000; when the tests run as root, which reads them anyway, the expectation
follows what the file system allows. `tests/memory_card.rs` checks the milestone 1
guarantees end to end on a folder laid out like a memory card: a dry run writes
nothing, the import fails only the unreadable file, a second run copies nothing, and
the source keeps its paths, bytes and modification times.

## Dependencies

- `adapter-blake3`, `adapter-clock`, `adapter-fs`, `files`, `import`, `naming`

- `clap` (ADR 0020 §6): `default-features = false`, features `std`, `help`, `usage`,
  `error-context`.
