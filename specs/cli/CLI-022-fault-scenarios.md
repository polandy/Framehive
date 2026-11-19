# CLI-022 – Fault scenarios

- Status: implemented
- Module: cli (`apps/cli`)
- Origin: T4, T7, T8, T9, N26, N5, N3, Q8, G7
- Depends on: CLI-001, CLI-005, CLI-007, CLI-008, CLI-015

## Purpose

The requirements (section 9.2) name faults an import must survive. This specification
states, for the scenarios that need no fault injection, what `framehive import` does,
so each one runs against the binary with real adapters.

## Behaviour

Each criterion names its scenario of requirements section 9.2 in its title. They add no
new behaviour: they combine the rules of the specifications they depend on.

| Scenario | Rule | Criterion |
|---|---|---|
| T4 Corrupt or truncated RAW file, broken EXIF data | A file whose metadata cannot be read is dated by its modification time (CORE-005) and still copied | CLI-022.1 |
| T7 Backup destination disappears | A backup destination that cannot be written fails per file (CLI-005), the card is not released (CLI-007), and a re-run completes the backup (IMPORT-009) | CLI-022.2, CLI-022.3 |
| T8 Same file on both card slots | The second slot's file is already imported or imported before (CLI-015, IMPORT-009) | CLI-022.4 |
| T8 Renamed file | A file renamed in the library is imported before | CLI-008.2 |
| T8 FAT32 timezone change | The history recognizes a file by its content, not by its modification time (IMPORT-009) | CLI-022.5 |
| T9 Camera file number overflows | Two files with the same name and date get distinct names (IMPORT-003) | CLI-022.6 |

A backup destination that is missing when the command starts is created, or stops the
command when it cannot be (CLI-005); the faults that need an injection point in the
binary (T1, T3, T6) are in CLI-025 (ADR 0048).

## Acceptance criteria

### CLI-022.1 – T4: damaged files do not stop the run
- **Given** a source with `DSCF0001.RAF`, a truncated `DSCF0002.RAF` and a
  `DSCF0003.JPG` whose EXIF data points outside the file, all modified at 2026-10-17
  14:05:09
- **When** `framehive import <source> --destination <destination>` runs
- **Then** every file is copied unchanged into `2026/2026-10-17/`, the lines of
  `DSCF0002.RAF` and `DSCF0003.JPG` end with `(dated by modification time: damaged: <reason>)`,
  the summary is `3 copied, 0 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out`,
  the last line is `The source can be formatted: every file is verified in 1 location.`,
  and the exit code is 0

### CLI-022.2 – T7: a backup destination that cannot be written
- **Given** a source with `DSCF0001.RAF` and `DSCF0002.RAF`, and a backup folder with a
  file `2026` where the target folder belongs
- **When** the command of CLI-022.1 runs with `--backup <backup>`
- **Then** both files are copied to the destination, each is followed by
  `  backup failed  <backup>: <reason>`, the backup summary is
  `backup <backup>: 0 backed up, 0 already backed up, 2 failed`, the output ends with
  `Do not format the source: not every file is verified in 2 locations.` and a line
  `  <file>: verified in 1 of 2 locations` for each file, and the exit code is 1

### CLI-022.3 – T7: a re-run completes the backup
- **Given** the run of CLI-022.2, after which the file `2026` was removed from the
  backup folder
- **When** the same command runs again
- **Then** each file is `imported before` and followed by `  backed up  <backup>`, the
  backup folder has both files, the last line is
  `The source can be formatted: every file is verified in 2 locations.`, and the exit
  code is 0

### CLI-022.4 – T8: the same file on both card slots
- **Given** sources `slot1` and `slot2`, each with the same `DSCF0001.RAF`
- **When** `framehive import slot1 slot2 --destination <destination>` runs
- **Then** the output has `copied  DSCF0001.RAF -> 2026/2026-10-17/DSCF0001.RAF`, and
  `already imported  DSCF0001.RAF -> 2026/2026-10-17/DSCF0001.RAF` or, when `slot2` is
  planned after the copy of `slot1` is recorded, `imported before` with the same paths;
  the summary after `slot1:` is
  `1 copied, 0 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out`, the one
  after `slot2:` is
  `0 copied, 1 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out`, the
  destination has one file, each source can be formatted, and the exit code is 0

### CLI-022.5 – T8: a modification time shifted by a time zone change
- **Given** a source with `clip.MOV` imported before, whose modification time was then
  shifted by one hour, as on a FAT32 card read in another time zone
- **When** the import runs again
- **Then** the output has `imported before  clip.MOV -> 2026/2026-10-17/clip.MOV` and
  the summary `0 copied, 1 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out`,
  the destination has one file, and the exit code is 0

### CLI-022.6 – T9: a camera file number that overflows
- **Given** a card with `DCIM/100_FUJI/DSCF0001.RAF`, `DCIM/100_FUJI/DSCF9999.RAF` and,
  after the overflow, `DCIM/101_FUJI/DSCF0001.RAF`, each with its own content, all
  captured on 2026-10-17
- **When** the import runs, and then runs again
- **Then** the first run prints
  `copied  DCIM/100_FUJI/DSCF0001.RAF -> 2026/2026-10-17/DSCF0001.RAF` and
  `copied  DCIM/101_FUJI/DSCF0001.RAF -> 2026/2026-10-17/DSCF0001-1.RAF`, keeps the
  content of each file, and exits with 0; the second run copies nothing and reports the
  three files as imported before

## Out of scope

- T1, T3 and T6: CLI-025 runs them through the fault-injection point of ADR 0048.
- T5 (a slow or hanging device): IMPORT-022 specifies its stall limit; CLI-025 is to run it.
- T2 (destination fills up): it needs a small file system of its own.
- T10 (cross-device staging): the import has no staging area; it copies into
  `<target>.part` beside the target (IMPORT-004).

## History

| Date | Change |
|---|---|
| 2026-11-19 | Created (review E20) |
| 2026-11-19 | T5 under Out of scope points to IMPORT-022, its stall limit |
