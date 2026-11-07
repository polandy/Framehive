# CLI-016 – Sources and destination of `framehive import`

- Status: implemented
- Module: cli (`apps/cli`)
- Origin: Q5, J3
- Depends on: CLI-001, CLI-010, CLI-013, CLI-015, IMPORT-011

## Purpose

`framehive import` names its sources and its destination as `cp` does: the last folder
is the destination, unless `--destination` names it, as `cp -t` does. The model is
familiar, and `framehive import <source> <destination>` of the first version keeps
working. The meaning of a folder on the command line never depends on `--profile`, so a
profile cannot name the destination (IMPORT-011): otherwise
`framehive import SD1 SD2 --profile p` could mean "import SD1 and SD2 into the
profile's destination" or "import SD1 onto the card SD2", depending on the profile.

## Behaviour

```
framehive import [OPTIONS] <source>... <destination>
framehive import [OPTIONS] --destination <folder> <source>...
```

**Without `--destination`**, the last positional argument is the destination and every
other one is a source, so at least two are needed. **With `--destination <folder>`**,
every positional argument is a source. A source is a folder, or
`camera:[<model>|<port>]` (CLI-013). The destination is created with its parents when
missing (CLI-001). `--destination` has no short form.

**Checks before any job.** These checks run before any source is scanned or any job
starts. Each failure prints the error to standard error, copies nothing, and exits
with 2. Each message names the problem, the affected path and the fix. They run in this
order, and the first failure stops the command:

1. The arguments themselves (the first three rows): they depend on nothing else.
2. The profile of `--profile` (CLI-010), which includes the row about `destination`.
3. The folders on the file system (the other rows), and the backup and video
   destinations against each source (CLI-005, CLI-012).

| Problem | Standard error |
|---|---|
| No positional argument, without `--destination` | `error: no source: give the folders or cameras to import from, then the folder to import into`, then `hint: framehive import /media/card ~/Pictures` |
| No positional argument, with `--destination <folder>` | `error: no source: give the folders or cameras to import from`, then `hint: framehive import --destination <folder> /media/card` |
| One positional argument, without `--destination` | `error: no destination: give the folder to import into after <source>, or with --destination`, then `hint: framehive import <source> ~/Pictures` |
| The last of two or more positional arguments is a `camera:` source, without `--destination` | `error: <camera:…> cannot be the destination; give the folder to import into last, or with --destination` |
| The profile has `destination` | `error: <path of the profile file>:<line>:<column>: destination is not read from a profile; give the folder to import into on the command line` (IMPORT-011) |
| A source does not exist | `error: the source <source> does not exist` |
| A source is not a folder | `error: the source <source> is not a folder; give the folder that holds it` |
| The destination is a source or lies inside one | `error: the destination <destination> lies inside the source <source>; choose a destination outside it` |
| A source lies inside another | `error: source <source> lies inside the source <other source>; give it once` |
| A source is given twice | `error: source <source> is given twice; give it once` |

Sources are named as given, except in the message about the destination, which names
both resolved (CLI-001.10). Two folders are compared by their resolved paths, so
`cardA` and `./cardA` are the same source, named by the later one. A `camera:` source
is given twice when its text after `camera:` is the same; cameras are not opened for
this check, and no folder lies inside a camera. A folder whose name starts with
`camera:` is written `./camera:…`, as a source and as the destination (CLI-013).
Arguments in messages and hints are printed as given; the shell quoting of the hints is
out of scope.

A source that passes these checks but cannot start, for example because it cannot be
read, does not keep the others from importing (CLI-015).

## Acceptance criteria

### CLI-016.1 – No source
- **Given** no positional argument
- **When** `framehive import` runs, and then `framehive import --destination /library`
- **Then** standard error of the first is exactly
  `error: no source: give the folders or cameras to import from, then the folder to import into`
  and `hint: framehive import /media/card ~/Pictures`; that of the second is exactly
  `error: no source: give the folders or cameras to import from` and
  `hint: framehive import --destination /library /media/card`; standard output is
  empty, and each exit code is 2

### CLI-016.2 – No destination
- **Given** a source `card` and no `--destination`
- **When** `framehive import card` runs
- **Then** standard error is exactly
  `error: no destination: give the folder to import into after card, or with --destination`
  and `hint: framehive import card ~/Pictures`, nothing is created, and the exit code
  is 2

### CLI-016.12 – A camera cannot be the destination
- **Given** no `--destination`
- **When** `framehive import /cardA camera:X-T5` runs
- **Then** standard error is exactly
  `error: camera:X-T5 cannot be the destination; give the folder to import into last, or with --destination`,
  standard output is empty, and the exit code is 2

### CLI-016.13 – A profile with a destination
- **Given** a profile `holiday.yaml` whose line 2 is `destination: ~/Pictures`
- **When** `framehive import <source> <destination> --profile holiday` runs
- **Then** standard error is exactly
  `error: <path of holiday.yaml>:2:1: destination is not read from a profile; give the folder to import into on the command line`,
  nothing is scanned, `<destination>` is not created, and the exit code is 2

### CLI-016.14 – The last folder is the destination
- **Given** sources `cardA` with `a.jpg` and `cardB` with `b.jpg`
- **When** `framehive import cardA cardB library` runs, and in another empty folder
  `framehive import --destination library cardA cardB`
- **Then** each `library` has the files of both sources, `cardB` is unchanged, and each
  exit code is 0

### CLI-016.15 – The arguments are checked before the profile, the profile before the folders
- **Given** no profile `missing`, and a profile `holiday` with `destination`
- **When** `framehive import card --profile missing` runs, and then
  `framehive import nowhere library --profile holiday`, where `nowhere` does not exist
- **Then** the first fails with the error of CLI-016.2, the second with that of
  CLI-016.13, and each exit code is 2

CLI-016.3 to CLI-016.5 are removed (2026-11-07): the destination of a profile and the
hint for the last folder as destination are gone, because the last folder is the
destination again.

### CLI-016.6 – A missing source starts no source
- **Given** a source `cardA` with `a.jpg`, and no `missing`
- **When** `framehive import cardA missing --destination library` runs
- **Then** standard error is exactly `error: the source missing does not exist`,
  `library` is not created, and the exit code is 2

### CLI-016.7 – A file as source
- **Given** a source `cardA` with `a.jpg`, and a file `b.jpg`
- **When** `framehive import cardA b.jpg --destination library` runs
- **Then** standard error is exactly
  `error: the source b.jpg is not a folder; give the folder that holds it`, `library` is
  not created, and the exit code is 2

### CLI-016.8 – The destination inside a later source
- **Given** sources `cardA` with `a.jpg` and `cardB` with `b.jpg`
- **When** `framehive import cardA cardB --destination cardB/library` runs
- **Then** standard error is exactly the message of the destination inside the source
  for the resolved `cardB/library` and `cardB`, nothing is copied, and the exit code is 2

### CLI-016.9 – A source inside another
- **Given** a source `card` with `DCIM/a.jpg`
- **When** `framehive import card/DCIM card --destination library` runs
- **Then** standard error is exactly
  `error: source card/DCIM lies inside the source card; give it once`, `library` is not
  created, and the exit code is 2

### CLI-016.10 – A source given twice
- **Given** a source `cardA` with `a.jpg`
- **When** `framehive import cardA ./cardA --destination library` runs
- **Then** standard error is exactly `error: source ./cardA is given twice; give it once`,
  `library` is not created, and the exit code is 2

### CLI-016.11 – A camera given twice
- **Given** no camera is connected
- **When** `framehive import camera: camera: --destination library` runs
- **Then** standard error is exactly `error: source camera: is given twice; give it once`,
  and the exit code is 2

## Out of scope

- The guided import, which asks for the sources and the destination (CLI-002).
- Shell quoting of the paths in the hint.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-07 | Created and implemented: decision C of ADR 0036, accepted by the owner |
| 2026-11-07 | Every source is checked before any job: CLI-016.6 to CLI-016.11 |
| 2026-11-07 | The `cp` model: the last folder is the destination unless `--destination` names it; profiles have no destination; the hint for the old habit is gone. CLI-016.1 and .2 changed, .3 to .5 removed, .12 to .15 added (ADR 0036, decision 8); accepted by the owner |
