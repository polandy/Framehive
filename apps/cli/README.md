<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# framehive-cli

Layer: `app` (architecture §3). Binary: `framehive`.

## Purpose

The command line of Framehive: `framehive import` (CLI-001, CLI-004, CLI-010, CLI-011), `framehive
import-log` (CLI-009), `framehive config init` (CLI-010) and, without a subcommand, the guided import (CLI-002, CLI-003, ADR 0024). It parses the arguments
with `clap` (builder API, ADR 0020 §6) and is one of the two places, with
`framehive-gui`, that know the concrete adapters and wire them to the ports.

## Usage

```
framehive
framehive import <source> <destination> [--backup <folder>]...
                 [--template <template>] [--job-code <text>]
                 [--on-conflict add_suffix|skip|fail] [--dry-run]
framehive import <source> [<destination>] --profile <name> [other options]
framehive import-log [<run>|last] [--format text|json|csv]
framehive config init
framehive --help
framehive --version
```

`framehive import` parses the name template, checks `--job-code` with
`import::JobCode` and `import::require_job_code` (IMPORT-006), resolves the source without symbolic
links, resolves the longest existing part of the destination and appends the missing
folders (rejecting `..` among them), and rejects a destination inside the source.
Each `--backup` folder (CLI-005) is resolved the same way and checked with
`import::check_backup_destinations` (IMPORT-007.11), also in a dry run. It scans and
plans, and checks the free space with `adapter-free-space` (CLI-011): a short file system
stops it with exit code 2 before any folder is created, a dry run after its lines.
Unless it is a dry run, it then creates the missing folders of the destination and of
each backup destination, and starts and executes the run with
`adapter-fs`, `adapter-blake3` and `adapter-clock`, in compatibility mode. The persisted sequence counters (CLI-004, ADR 0029) come from
`adapter-sequence-counters`, and the import history (CLI-008, ADR 0031) from
`adapter-sqlite`, both in the state folder; the command stops when the state folder is
unknown (no absolute `XDG_STATE_HOME` and no home folder). The run gets the resolved paths
above, so a destination has the same path in every run, which is how the history
identifies it (IMPORT-009).

## Import profiles

`framehive import --profile <name>` (CLI-010, `src/profiles.rs`) checks the name
(ADR 0032), finds the configuration folder (a pure function of the platform,
`XDG_CONFIG_HOME` and `HOME`, tested for Linux and macOS) and reads
`profiles/<name>.yaml` through the `FileSystem` port with `import::ImportProfile::read`
and `adapter-yaml`, with `HOME` for `~`, before anything is scanned. Each setting comes
from the command line, else the profile, else the default of CLI-001; `--backup`
replaces the profile's list. Only `--job-code` gets the unused job code warning. Every
failure is `error: …` with exit code 2. `framehive config init` (`src/config_init.rs`)
writes `profiles/example.yaml` as `<file>.part` renamed without replacing, so an
existing file is kept (exit code 1).

## Import log

`framehive import-log` (CLI-009) opens the import history of the state folder with
`SqliteImportHistory::read_only`, so it never creates the history or the state folder.
Without `<run>` it prints one line per run from `ImportHistory::list_runs`; with a run
number or `last` it prints `ImportHistory::read_run_log` as text, JSON or RFC 4180 CSV
(`src/import_log/`). The JSON and CSV writers are own code (ADR 0009). Paths are printed
lossy, with U+FFFD for each byte that is not valid UTF-8. `--format` without `<run>` is a
usage error. A run that does not exist, `last` without runs, a history that cannot be
used and an unknown state folder print `error: …` and exit with 2.

## Guided import

`framehive` without a subcommand runs the dialog of CLI-002 and CLI-003 in
`src/guided/`: it waits for a memory card (`devices::DeviceWatch` over `adapter-mounts`, polled once a second
while it waits for a line), reads the card with `import::scan_folder`, asks for the
template (with a preview from `ImportRun::plan_first`) and the job code, shows the plan
summary, reserves the sequence numbers with `ImportRun::start` on Enter (numbers that
do not fit lead back to the template step), and runs `ImportRun::execute` with a progress sink that redraws one line and
stops the run between files when the card is pulled (IMPORT-004.13). The dialog is a
loop over input events (a line, a tick per poll interval, the end of input) from a
thread that reads standard input. Backup destinations (CLI-006) are chosen with `b` at
the plan summary and passed to the run; one on the card, or one that
`import::check_backup_destinations` rejects, blocks Enter like a destination on the
card, and the report adds the files with a failed backup and the backup summary lines.
The import history is `adapter-sqlite` in the state folder; the destinations are resolved
with `paths::canonical_folder` (through `Ports::resolve_folder`) before each plan, and a
history that cannot be used is reported as `The import history <path> cannot be used:
<reason>`.
The destination, the backup destinations, the template and the recent job codes are
remembered in `$XDG_STATE_HOME/framehive/guided-import.txt` (ADR 0025, format version 2
of ADR 0030; version 1 is still read), written through the `FileSystem` port as
`<file>.part`, then renamed. It exits with 0
when the person quits, and with 2 when it cannot start: standard input or output is not
a terminal, or the home folder or the current folder is unknown.

## Report and exit codes

The report texts live here, not in the import module. Standard output gets one line
per file as soon as it is done (`copied`, `would copy`, `already imported`,
`imported before`, `duplicate`, `skipped`, `failed`), each followed by one indented line
per backup destination (`backed up`, `already backed up`, `backed up before`,
`backup failed`) that names the folder
as given, the files and folders the scan left out (`left out`), a summary line, and one
`backup <folder>: …` line per backup destination; in a dry run `copied` reads
`would copy`, in the summary as well, and there are no backup lines. The format
readiness (CLI-007, `format_readiness_lines`) ends the report of a run that is not a
dry run: `framehive import` says `source`, the guided import says `card` and prints it
before the eject question. Errors that stop the command go to standard error as
`error: <message>`, and so does an import history that fails during the run, after the
summary. A job code that the template does not use is accepted and reported on
standard error as `warning: <message>`; there is no other log output yet.

| Code | Meaning |
|---|---|
| 0 | Every file was copied, already imported, imported before, a duplicate, skipped or left out |
| 1 | A file failed, or a folder could not be read (`ImportSummary::has_failures`), a file failed at a backup destination, or the import history failed during the run |
| 2 | The import did not start: invalid arguments, template or job code, missing source, destination inside the source or not creatable, an invalid or not creatable backup destination, sequence numbers that do not fit, counters that cannot be read or stored, an unknown state folder, an import history that cannot be used, a file system without enough space, or a profile that cannot be found, read or used (CLI-010) |

## Modules

| Module | Meaning |
|---|---|
| `arguments` | The `clap` command line and the parsed `Invocation` |
| `paths` | Source and destination resolution, the inside check, creating the destination |
| `import_command` | Wires the adapters to the import run (`run.rs`, tested in-crate with `MemoryFreeSpace`) and maps the result to the exit code |
| `report` | The `ProgressSink` that prints the report lines and the summary |
| `profiles` | The configuration folder, profile names, and reading the profile of `--profile` (CLI-010) |
| `config_init` | `framehive config init`: the example profile (CLI-010) |
| `import_log` | `framehive import-log` (CLI-009): reading the history, and the `text`, `json` and `csv` writers |
| `format_readiness` | The format readiness lines (CLI-007), shared by `framehive import` and the guided import |
| `guided` | The guided import (CLI-002, CLI-003): `dialog` (prompts, waiting, remembered answers), `card` (summary, import, eject), `summary` (plan summary, blockers), `template` (template step, job code), `progress` (progress line, stop on a pulled card), `format` (sizes, rate, time left), `state` (state file), `events` (input events) |

## Tests

`tests/` runs the real binary (`CARGO_BIN_EXE_framehive`) in a temporary folder with
`TZ=UTC`, with test files from `metadata::testing`. Unreadable files and folders use
mode 000; when the tests run as root, which reads them anyway, the expectation
follows what the file system allows. `tests/import_profiles.rs` reads import profiles (IMPORT-011) with the real YAML
parser, including every profile shown in `docs/` and `specs/` and the example of
`framehive config init`. `tests/import_profile_option.rs` and `tests/config_init.rs`
(CLI-010) set `HOME` to a temporary folder and unset `XDG_CONFIG_HOME`. `tests/memory_card.rs` checks the milestone 1
guarantees end to end on a folder laid out like a memory card: a dry run writes
nothing, the import fails only the unreadable file, a second run copies nothing, and
the source keeps its paths, bytes and modification times. `tests/import_log.rs` reads
the log after real imports, or of a history written with `adapter-sqlite`.

The guided import is tested in the crate (`src/guided/tests/`) with golden transcripts:
the dialog runs against scripted input steps, `MemoryFileSystem`, `MemoryDeviceSource`
and a `ManualClock`; `tests/guided_import.rs` checks the binary without a terminal.

## Dependencies

- `adapter-blake3`, `adapter-clock`, `adapter-fs`, `adapter-mounts`,
  `adapter-sequence-counters`, `adapter-sqlite`, `adapter-yaml`, `config`, `devices`,
  `domain`, `files`, `history`, `import`, `naming`

- `clap` (ADR 0020 §6): `default-features = false`, features `std`, `help`, `usage`,
  `error-context`.
