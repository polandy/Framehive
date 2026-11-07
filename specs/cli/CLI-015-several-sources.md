# CLI-015 – Several sources in one import command

- Status: implemented
- Module: cli (`apps/cli`)
- Origin: Q5, J3
- Depends on: CLI-001, CLI-007, CLI-016, IMPORT-002, IMPORT-015

## Purpose

A photographer with two cards, or a card and a camera, imports both with one command,
and the imports run at the same time when they do not compete for a device or a disk.

## Behaviour

`framehive import <source>... <destination>` and
`framehive import --destination <folder> <source>...` take one or more sources
(CLI-001, CLI-016).

**One job per source.** Each source is imported by an import job of its own
(IMPORT-015), with the same destinations and settings. The command checks, scans and
plans the sources one by one in the order given, and enqueues each source's job as soon
as it is planned, so its sequence numbers follow those of the source before it
(IMPORT-002). Jobs whose sources are on separate devices and whose destinations share
no file system run in parallel; the others run one after another (IMPORT-015,
"Resources").

**Arguments.** The last folder is the destination unless `--destination` names it, with
or without `--profile` (CLI-016). Arguments that hold for every source, such
as the template or the job code, are checked once, before any source; so are the sources themselves (CLI-016). An
error there starts no source. A source that passes these checks but cannot start, for
example because it cannot be read, its camera cannot be opened, or its destination is short of space, does not keep the others from importing: each of its errors goes to standard
error as `error: <source as given>: <message>` as soon as it is found, not after the
other sources ended.

**Report.** The file lines of CLI-001 are unchanged; those of parallel jobs may
interleave. Each source's summary line, with the lines that follow it (CLI-005,
CLI-007), is printed when that source's job ends, after a line `<source as given>:`.
With one source there is no such line, and the output is that of CLI-001.

**Dry run.** A dry run of several sources counts the sequence numbers and target names
across the sources in memory (IMPORT-016), so it shows what a real run would do. As
with one source (CLI-001, CLI-004), it creates no file or folder and stores no number.

**Exit code.** The highest of the sources' exit codes (CLI-001): 2 when a source did not
start, else 1 when a file failed, else 0.

## Acceptance criteria

### CLI-015.1 – Two sources
- **Given** a source `cardA` with `a.jpg` and a source `cardB` with `b.jpg`, both
  captured at 2026-10-17 14:05:09, and an empty destination
- **When** `framehive import cardA cardB --destination <destination>` runs
- **Then** both files are copied, the output has the line `cardA:` followed by
  `1 copied, 0 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out`, and the
  line `cardB:` followed by the same summary, and the exit code is 0

### CLI-015.2 – The worst exit code of the sources
- **Given** a source `cardA` with a file that can be copied, and a source `cardB` with a
  file whose target name is taken by another file
- **When** both are imported with one command and `--on-conflict fail`
- **Then** the file of `cardA` is copied, and the exit code is 1

### CLI-015.3 – A source that cannot start
- **Given** a source `cardA` with `a.jpg`, and a source `camera:disk:<missing folder>`,
  a camera that cannot be opened for any user
- **When** `framehive import cardA camera:disk:<missing folder> --destination <destination>`
  runs
- **Then** `a.jpg` is copied, standard error has a line that starts with
  `error: camera:disk:<missing folder>: `, and the exit code is 2

### CLI-015.4 – One source prints no source line
- **Given** the source of CLI-001.1
- **When** it is imported alone
- **Then** the output has no line naming the source

### CLI-015.5 – Sources continue each other's sequence numbers
- **Given** sources `cardA` with two pictures and `cardB` with one, the template
  `{seq_stored:3}.{ext}` and the stored counter at 0
- **When** `framehive import cardA cardB --destination <destination>` runs
- **Then** the pictures of `cardA` get `001` and `002`, and the picture of `cardB` `003`

### CLI-015.6 – An argument error starts no source
- **Given** two sources and the template `{job}/{name}.{ext}` without `--job-code`
- **When** the command runs
- **Then** standard error names `{job}`, no destination is created, and the exit code
  is 2

### CLI-015.7 – With a profile, the last folder is still the destination
- **Given** a profile `holiday` with the template `{name}.{ext}`, and the sources
  `cardA` with `a.jpg` and `cardB` with `b.jpg`
- **When** `framehive import cardA cardB library --profile holiday` runs
- **Then** both pictures are imported into `library` by the template of the profile,
  and nothing is written to `cardB`

### CLI-015.8 – With `--destination` and a profile, every folder is a source
- **Given** the profile and the sources of CLI-015.7
- **When** `framehive import cardA cardB --destination library --profile holiday` runs
- **Then** both pictures are imported into `library` by the template of the profile

### CLI-015.9 – The error of a source comes at once
- **Given** a source `camera:disk:<missing folder>`, a camera that cannot be opened for
  any user, and a source `cardA` with `a.jpg`
- **When** `framehive import camera:disk:<missing folder> cardA --destination <destination>`
  runs with standard output and standard error written to one file
- **Then** the line that starts with `error: camera:disk:<missing folder>: ` comes before
  the line `cardA:`, and the exit code is 2

### CLI-015.10 – A dry run of several sources
- **Given** a source `cardA` with `a.jpg` and `b.jpg`, a source `cardB` with `c.jpg`,
  and the stored counter at 0
- **When** `framehive import cardA cardB --destination library --dry-run --template '{seq_stored:3}.{ext}'`
  runs
- **Then** the output has `would copy` lines to `001.jpg` and `002.jpg` for `cardA` and
  to `003.jpg` for `cardB`, the stored counter is still 0, `library` does not exist, and
  the exit code is 0

## Out of scope

- Queueing a newly inserted card in the guided import while a job runs: a later step
  of J2.
- Progress across all running jobs (N12, J4): the TUI concept and the GUI.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-07 | Created (Q5, J3, ADR 0036) |
| 2026-11-07 | Implemented (J3, Q5); the last folder is the destination also with a profile |
| 2026-11-07 | Every folder is a source, also with a profile; `--destination` names the destination (CLI-016, ADR 0036); CLI-015.7 changed, CLI-015.8 added |
| 2026-11-07 | The sources are checked before any job (CLI-016): CLI-015.3 uses a source that cannot be read |
| 2026-11-07 | The error of a source that cannot start is printed at once: CLI-015.9 |
| 2026-11-07 | A dry run of several sources counts in memory (IMPORT-016): CLI-015.10 |
| 2026-11-07 | CLI-015.9 uses a camera that cannot be opened, which fails for every user, also root |
| 2026-11-07 | CLI-015.3 uses a camera that cannot be opened, which fails for every user, also root |
| 2026-11-07 | The `cp` model (CLI-016, ADR 0036, decision 8): the last folder is the destination again, also with a profile; CLI-015.7 and CLI-015.8 changed |
