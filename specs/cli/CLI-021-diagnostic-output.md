# CLI-021 – Diagnostic output

- Status: implemented
- Module: cli (`apps/cli`)
- Origin: N18
- Depends on: CLI-001

## Purpose

When an import from a camera hangs or a file system takes a fallback, there is nothing
to look at or to attach to an error report (review E28, architecture §6). Every
`framehive` command writes diagnostic lines to standard error, as detailed as
`--verbose` asks (ADR 0046).

## Behaviour

`-v` or `--verbose` is accepted before or after the subcommand and may be repeated:

| Given | Shown |
|---|---|
| no `--verbose` | warnings and errors |
| once | also information, such as each file of an import |
| twice | also details, such as a rename without a hard link |
| three times or more | everything |

A diagnostic line has the level, the spans it was written in with their fields, the
place in the code and the message with its fields, and no time:

```
 INFO import_job{source=/card}:file{path=DSCF0001.RAF}: import::run: file done outcome=Copied { … }
```

Diagnostic lines go to standard error only; the report on standard output, the
`error:` and `hint:` lines and the exit codes are the same with and without `--verbose`.

## Acceptance criteria

### CLI-021.1 – Without --verbose only warnings are shown
- **Given** an event of each level
- **When** it is written without `--verbose`
- **Then** only the warning is shown, and a plain import writes no ` INFO ` line

### CLI-021.2 – Each --verbose shows one level more
- **Given** an event of each level
- **When** it is written with `--verbose` once, twice and three times
- **Then** once shows information but no details, twice details but no trace, three
  times everything

### CLI-021.3 – A line names its spans and has no time
- **Given** an information event inside the span `import_job` with `source=/card`
- **When** it is written with `--verbose`
- **Then** the line is ` INFO import_job{source="/card"}: <place>: <message>`

### CLI-021.4 – A verbose import names the job and each file
- **Given** the source of CLI-001.1
- **When** `framehive import <source> <destination> --verbose` runs
- **Then** standard error has an information line in the spans
  `import_job{source=<source>}:file{path=DSCF0001.RAF}` with `file done` and
  `outcome=Copied`, and a line `import job done end=Finished`; standard output has
  no diagnostic line, and the exit code is 0

### CLI-021.5 – --verbose works before the subcommand
- **Given** a state folder without an import history
- **When** `framehive -v import-log` runs
- **Then** the exit code is 0

## Out of scope

- A log file in the state folder and a diagnostic export for error reports: they can
  follow once the events have proven useful.
- Filtering by crate or by an environment variable such as `RUST_LOG`.

## History

| Date | Change |
|---|---|
| 2026-11-19 | Created (review E28) |
