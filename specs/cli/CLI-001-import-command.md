# CLI-001 – `framehive import`

- Status: accepted
- Module: cli (`apps/cli`)
- Origin: N18, X1, N26
- Depends on: IMPORT-001, IMPORT-003, IMPORT-004, IMPORT-005, CORE-005

## Purpose

The first way to use Framehive is a command that imports a folder or a mounted memory
card into a library folder, without a graphical interface. It suits scripts, and it
shows every decision the import makes.

## Behaviour

```
framehive import <source> <destination> [--template <template>]
                 [--on-conflict add_suffix|skip|fail] [--dry-run]
```

| Argument | Meaning | Default |
|---|---|---|
| `<source>` | Folder to import from (IMPORT-005) | required |
| `<destination>` | Folder to import into; created with its parents when missing | required |
| `--template` | Name template (IMPORT-001) | `{date:%Y}/{date:%Y-%m-%d}/{name}.{ext}` |
| `--on-conflict` | Conflict policy (IMPORT-003) | `add_suffix` |
| `--dry-run` | Make the plan and print it, copy nothing | off |

Compatibility mode (IMPORT-001) is on. `framehive --help` and `framehive import --help`
describe the arguments; `framehive --version` prints `framehive <version>`.

**Flow.** The command checks its arguments and the template, scans the source, makes
the import plan with conflicts resolved, and copies each file with a verified copy
(IMPORT-004), in scan order.

**Report.** Standard output gets one line per file as soon as the file is done, then a
summary. Paths are relative to the source and to the destination.

| Line | When |
|---|---|
| `copied  <source path> -> <target path>` | Copied and verified |
| `already imported  <source path> -> <target path>` | Identical content at the target (IMPORT-003) |
| `duplicate  <source path> = <earlier source path>` | Same content as an earlier file of the plan |
| `skipped  <source path>: <reason>` | Name conflict under `skip` |
| `failed  <source path>: <reason>` | Any failure of that file |
| `left out  <source path>: <reason>` | Left out by the scan (IMPORT-005) |

A copied or planned file dated by its modification time ends with
`(dated by modification time: <reason>)` (CORE-005). With `--dry-run`, `copied` reads
`would copy`, and no file or folder is created.

The summary has one line:
`<n> copied, <n> already imported, <n> duplicates, <n> skipped, <n> failed, <n> left out`.

Errors that stop the command, and log messages, go to standard error.

**Exit codes.**

| Code | Meaning |
|---|---|
| 0 | Every file was copied, already imported, a duplicate, skipped or left out |
| 1 | The import ran, and at least one file failed or a folder could not be read |
| 2 | The import did not start: invalid arguments, an invalid template, a missing source, a destination inside the source, or a destination that cannot be created |

## Acceptance criteria

### CLI-001.1 – Import with the defaults
- **Given** a source with `DSCF0001.RAF` captured at 2026-10-17 14:05:09, and an empty
  destination
- **When** `framehive import <source> <destination>` runs
- **Then** the destination has `2026/2026-10-17/DSCF0001.RAF`, the output has the line
  `copied  DSCF0001.RAF -> 2026/2026-10-17/DSCF0001.RAF` and the summary
  `1 copied, 0 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out`, and
  the exit code is 0

### CLI-001.2 – A second run copies nothing
- **Given** the import of CLI-001.1 has run
- **When** the same command runs again
- **Then** the output has `already imported  DSCF0001.RAF -> 2026/2026-10-17/DSCF0001.RAF`,
  no file is changed, and the exit code is 0

### CLI-001.3 – Own template
- **Given** the source of CLI-001.1
- **When** `framehive import <source> <destination> --template '{date:%Y%m%d}-{name}.{ext}'`
  runs
- **Then** the destination has `20261017-DSCF0001.RAF`

### CLI-001.4 – Conflict policy
- **Given** the destination has `2026/2026-10-17/DSCF0001.RAF` with other content
- **When** the import of CLI-001.1 runs with `--on-conflict skip`, and again with
  `--on-conflict fail`
- **Then** the first prints `skipped  DSCF0001.RAF: name conflict` and exits with 0, the
  second prints `failed  DSCF0001.RAF: name conflict` and exits with 1, and the existing
  file is unchanged

### CLI-001.5 – Dry run
- **Given** the source of CLI-001.1 and a destination that does not exist
- **When** the command runs with `--dry-run`
- **Then** the output has `would copy  DSCF0001.RAF -> 2026/2026-10-17/DSCF0001.RAF`,
  the destination still does not exist, and the exit code is 0

### CLI-001.6 – A failed file
- **Given** a source with `a.jpg`, `b.jpg` and `c.jpg`, and `b.jpg` cannot be read
- **When** the import runs
- **Then** `a.jpg` and `c.jpg` are copied, the output has `failed  b.jpg: <reason>` and a
  summary with `2 copied` and `1 failed`, and the exit code is 1

### CLI-001.7 – Files left out and dated by modification time
- **Given** a source with `notes.txt` and `clip.MOV` modified at 2026-10-17 14:05:09
  local time
- **When** the import runs
- **Then** the output has `left out  notes.txt: unsupported type` and
  `copied  clip.MOV -> 2026/2026-10-17/clip.MOV (dated by modification time: format not read)`,
  and the exit code is 0

### CLI-001.8 – Invalid template
- **Given** the template `{date:%Y}/{camera}.{ext}`
- **When** the command runs with it
- **Then** standard error names `{camera}` and column 11, nothing is copied, and the
  exit code is 2

### CLI-001.9 – Missing source
- **Given** a source path that does not exist
- **When** the command runs
- **Then** standard error names the path, and the exit code is 2

### CLI-001.10 – Destination inside the source
- **Given** the destination `<source>/library`
- **When** the command runs
- **Then** standard error names both paths, nothing is copied, and the exit code is 2

### CLI-001.11 – Invalid arguments
- **Given** `--on-conflict overwrite`, or a missing destination
- **When** the command runs
- **Then** standard error says what is wrong and points to `--help`, and the exit code
  is 2

### CLI-001.12 – Missing destination is created
- **Given** a destination `<dir>/new/library` whose parents do not exist
- **When** the import of CLI-001.1 runs
- **Then** the folders are created, the file is copied, and the exit code is 0

### CLI-001.13 – Unreadable folder in the source
- **Given** a source with `DSCF0001.RAF` and a folder `locked` that cannot be read
- **When** the import runs
- **Then** `DSCF0001.RAF` is copied, the output has `left out  locked: <reason>`, and the
  exit code is 1

## Out of scope

- Profiles and the configuration file (N8, N25), `--profile` (N18): later specs.
- Devices: detecting and naming cards and cameras as sources (Q1, Q2): the device spec.
- Backup destinations (I2), the import log (N6) and resuming (N4): later specs.
- Progress with throughput and remaining time (N12): the TUI concept and the GUI.
- Translated output (X2): text is English until a spec for languages exists.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-10-31 | Initial draft (milestone 1) |
| 2026-10-31 | Accepted by the owner |
