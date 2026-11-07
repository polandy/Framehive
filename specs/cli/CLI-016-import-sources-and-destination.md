# CLI-016 – Sources and destination of `framehive import`

- Status: implemented
- Module: cli (`apps/cli`)
- Origin: Q5, J3
- Depends on: CLI-001, CLI-010, CLI-013, CLI-015

## Purpose

Every folder on the command line of `framehive import` is a source, and the destination
is always named by an option. A rule that made the last folder the destination, or not,
depending on `--profile`, could import one card onto another without a warning: with
`framehive import SD1 SD2 --profile p`, SD2 would be the destination. Naming the
destination with `--destination`, as `cp -t` does, removes that guess.

## Behaviour

```
framehive import [OPTIONS] <source>... [--destination <folder>]
```

**Sources.** Every positional argument is a source: a folder, or `camera:[<model>|<port>]`
(CLI-013). Its meaning never depends on another option.

**Destination.** `--destination <folder>` names the folder to import into; it is created
with its parents when missing (CLI-001). Without it, the destination of the profile
given with `--profile` is used (CLI-010).

**Checks before any job.** These checks run before any source is scanned or any job
starts. Each failure prints the error to standard error, copies nothing, and exits
with 2. Each message names the problem, the affected path and the fix.

| Problem | Standard error |
|---|---|
| No source | `error: no source: give the folders or cameras to import from`, then `hint: framehive import /media/card --destination ~/Pictures` |
| No destination | `error: no destination: give --destination <folder>` |
| No destination, with `--profile <name>` | `error: no destination: give --destination <folder>, or set destination in the profile <name>: <path of the profile file>` |

**The old habit.** When the destination is missing and two or more sources are given,
the last of them a folder, someone probably meant that folder as the destination. A
second line then shows the command that does so:
`hint: every folder is a source; to import into <last folder>, write: framehive import <other sources> --destination <last folder>`.
The arguments are printed as given. The hint depends only on the arguments, never on
the file system.

## Acceptance criteria

### CLI-016.1 – No source
- **Given** no source
- **When** `framehive import --destination /library` runs
- **Then** standard error is exactly
  `error: no source: give the folders or cameras to import from` and
  `hint: framehive import /media/card --destination ~/Pictures`, standard output is
  empty, and the exit code is 2

### CLI-016.2 – No destination
- **Given** a source and neither `--destination` nor `--profile`
- **When** the command runs
- **Then** standard error is exactly `error: no destination: give --destination <folder>`,
  nothing is created, and the exit code is 2

### CLI-016.3 – No destination in the profile
- **Given** a profile `holiday` without a destination
- **When** `framehive import <source> --profile holiday` runs
- **Then** standard error is exactly
  `error: no destination: give --destination <folder>, or set destination in the profile holiday: <path of holiday.yaml>`,
  and the exit code is 2

### CLI-016.4 – The last folder as destination, in the old way
- **Given** no `--destination` and no `--profile`
- **When** `framehive import /cardA /cardB /library` runs
- **Then** standard error is exactly the error of CLI-016.2 and
  `hint: every folder is a source; to import into /library, write: framehive import /cardA /cardB --destination /library`,
  nothing is copied, and the exit code is 2

### CLI-016.5 – No hint for a camera
- **Given** no `--destination`
- **When** `framehive import /cardA camera:` runs
- **Then** standard error is exactly the error of CLI-016.2, without a hint

## Out of scope

- The guided import, which asks for the sources and the destination (CLI-002).
- Shell quoting of the paths in the hint.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-07 | Created and implemented: decision C of ADR 0036, accepted by the owner |
