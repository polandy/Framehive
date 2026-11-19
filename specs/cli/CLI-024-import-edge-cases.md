# CLI-024 – Version, destination errors and memory card

- Status: implemented
- Module: cli (`apps/cli`)
- Origin: N18
- Depends on: CLI-001

## Purpose

Four system tests of `framehive import` had no acceptance criterion (review E21,
ADR 0010). This spec gives them one. CLI-001 holds the behavior.

## Behaviour

As CLI-001: `framehive --version` prints `framehive <version>`, a destination that
cannot be created stops the command with exit code 2, and a memory card is imported like
any other folder.

## Acceptance criteria

### CLI-024.1 – Version and help
- **Given** the binary
- **When** `framehive --version` runs, and `framehive import --help`
- **Then** the first prints `framehive <version>` and exits with 0, the second describes
  `--profile`, `--template`, `--job-code`, `--on-conflict` and `--dry-run` and both forms
  of the folders

### CLI-024.2 – A destination that cannot be created
- **Given** a destination `<file>/library`, where `<file>` is a file
- **When** the import of CLI-001.1 runs
- **Then** standard error names `library`, and the exit code is 2

### CLI-024.3 – Parent folder steps in a missing destination
- **Given** a destination `<root>/new/../library` whose folder `new` does not exist
- **When** the import of CLI-001.1 runs
- **Then** standard error names `..`, the folder `new` is not created, and the exit code
  is 2

### CLI-024.4 – A memory card is imported once and left unchanged
- **Given** a folder laid out like a memory card, with a RAW and JPEG pair, a video, the
  hidden folder `.Trashes`, and a file at the target of the video
- **When** the import runs with `--on-conflict fail`, with `--dry-run` first, and a
  second time
- **Then** the dry run writes nothing and foresees the conflict, the import copies the
  pair and fails only the video and leaves out `.Trashes`, the second run finds the pair
  imported before, the exit code is 1, and the source is never changed

## Out of scope

- The behavior itself: CLI-001.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-19 | Initial version, implemented (review finding E21) |
