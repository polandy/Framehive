<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# framehive-cli

Layer: `app` (architecture §3). Binary: `framehive`.

## Purpose

The command line of Framehive: `framehive import` (CLI-001) and, without a
subcommand, the guided import (CLI-002, CLI-003, ADR 0024). It parses the arguments
with `clap` (builder API, ADR 0020 §6) and is one of the two places, with
`framehive-gui`, that know the concrete adapters and wire them to the ports.

## Usage

```
framehive
framehive import <source> <destination> [--template <template>]
                 [--job-code <text>] [--on-conflict add_suffix|skip|fail]
                 [--dry-run]
framehive --help
framehive --version
```

`framehive import` parses the name template, checks `--job-code` with
`import::JobCode` and `import::require_job_code` (IMPORT-006), resolves the source without symbolic
links, resolves the longest existing part of the destination and appends the missing
folders (rejecting `..` among them), and rejects a destination inside the source.
Unless it is a dry run, it then creates the missing folders of the destination and
runs `import::ImportRun` with `adapter-fs`, `adapter-blake3` and `adapter-clock`, in
compatibility mode.

## Guided import

`framehive` without a subcommand runs the dialog of CLI-002 and CLI-003 in
`src/guided/`: it waits for a memory card (`devices::DeviceWatch` over `adapter-mounts`, polled once a second
while it waits for a line), reads the card with `import::scan_folder`, asks for the
template (with a preview from `ImportRun::plan_first`) and the job code, shows the plan
summary, and runs `ImportRun::execute` with a progress sink that redraws one line and
stops the run between files when the card is pulled (IMPORT-004.13). The dialog is a
loop over input events (a line, a tick per poll interval, the end of input) from a
thread that reads standard input. The destination, the template and the recent job
codes are remembered in `$XDG_STATE_HOME/framehive/guided-import.txt` (ADR 0025),
written through the `FileSystem` port as `<file>.part`, then renamed. It exits with 0
when the person quits, and with 2 when it cannot start: standard input or output is not
a terminal, or the home folder or the current folder is unknown.

## Report and exit codes

The report texts live here, not in the import module. Standard output gets one line
per file as soon as it is done (`copied`, `would copy`, `already imported`,
`duplicate`, `skipped`, `failed`), the files and folders the scan left out
(`left out`), and a summary line; in a dry run `copied` reads `would copy`, in the
summary as well. Errors that stop the command go to standard error as
`error: <message>`. A job code that the template does not use is accepted and
reported on standard error as `warning: <message>`; there is no other log output
yet.

| Code | Meaning |
|---|---|
| 0 | Every file was copied, already imported, a duplicate, skipped or left out |
| 1 | A file failed, or a folder could not be read (`ImportSummary::has_failures`) |
| 2 | The import did not start: invalid arguments, template or job code, missing source, destination inside the source or not creatable |

## Modules

| Module | Meaning |
|---|---|
| `arguments` | The `clap` command line and the parsed `Invocation` |
| `paths` | Source and destination resolution, the inside check, creating the destination |
| `import_command` | Wires the adapters to the import run and maps the result to the exit code |
| `report` | The `ProgressSink` that prints the report lines and the summary |
| `guided` | The guided import (CLI-002, CLI-003): `dialog` (prompts, waiting, remembered answers), `card` (summary, import, eject), `template` (template step, job code), `progress` (progress line, stop on a pulled card), `format` (sizes, rate, time left), `state` (state file), `events` (input events) |

## Tests

`tests/` runs the real binary (`CARGO_BIN_EXE_framehive`) in a temporary folder with
`TZ=UTC`, with test files from `metadata::testing`. Unreadable files and folders use
mode 000; when the tests run as root, which reads them anyway, the expectation
follows what the file system allows. `tests/memory_card.rs` checks the milestone 1
guarantees end to end on a folder laid out like a memory card: a dry run writes
nothing, the import fails only the unreadable file, a second run copies nothing, and
the source keeps its paths, bytes and modification times.

The guided import is tested in the crate (`src/guided/tests/`) with golden transcripts:
the dialog runs against scripted input steps, `MemoryFileSystem`, `MemoryDeviceSource`
and a `ManualClock`; `tests/guided_import.rs` checks the binary without a terminal.

## Dependencies

- `adapter-blake3`, `adapter-clock`, `adapter-fs`, `adapter-mounts`, `devices`, `domain`,
  `files`, `import`, `naming`

- `clap` (ADR 0020 §6): `default-features = false`, features `std`, `help`, `usage`,
  `error-context`.
