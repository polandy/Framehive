# CLI-005 – Backup destinations in `framehive import`

- Status: implemented
- Module: cli (`apps/cli`)
- Origin: I2, I5, N18
- Depends on: CLI-001, IMPORT-007

## Purpose

`framehive import` (CLI-001) writes each file to backup destinations as well
(IMPORT-007), so a script can import a memory card to the library and to a backup drive
in one run, and can tell from the exit code whether every copy is verified.

## Behaviour

```
framehive import <source> <destination> [--backup <folder>]... [other arguments]
```

`--backup <folder>` adds a backup destination. It can be given more than once; the
backup destinations keep the order of the command line. Without `--backup`, the
command behaves as CLI-001 describes.

**Report.** After the line of a file that is backed up (IMPORT-007, "Which files are
backed up"), standard output gets one line per backup destination, indented by two
spaces. `<backup>` is the folder as given on the command line.

| Line | When |
|---|---|
| `  backed up  <backup>` | Copied and verified at that backup destination |
| `  already backed up  <backup>` | Identical content at the backup target |
| `  backup failed  <backup>: <reason>` | Any failure at that backup destination |

After the summary of CLI-001, standard output gets one line per backup destination:
`backup <backup>: <n> backed up, <n> already backed up, <n> failed`.

With `--dry-run`, the backup destinations are checked as before a run (IMPORT-007.11),
nothing is written to them, and no backup lines are printed.

**Exit codes.** A failure at a backup destination makes the exit code 1, as a failed
file does (CLI-001). An invalid backup destination (IMPORT-007.11), or one that cannot
be created, stops the command before any file is copied, with exit code 2.

## Acceptance criteria

### CLI-005.1 – Import with a backup destination
- **Given** a source with `DSCF0001.RAF` taken on 2026-10-17, and an empty destination
  and backup folder
- **When** the user runs `framehive import <source> <destination> --backup <backup>`
- **Then** both folders have `2026/2026-10-17/DSCF0001.RAF`, the output has the line
  `copied  DSCF0001.RAF -> 2026/2026-10-17/DSCF0001.RAF` followed by
  `  backed up  <backup>`, the summary line is followed by
  `backup <backup>: 1 backed up, 0 already backed up, 0 failed`, and the exit code is 0

### CLI-005.2 – A re-run fills a new backup destination
- **Given** the folders after CLI-005.1 without its import history, and a second, empty
  backup folder
- **When** the user runs the command again with `--backup <backup> --backup <second>`
- **Then** the output has `already imported  DSCF0001.RAF -> 2026/2026-10-17/DSCF0001.RAF`,
  then `  already backed up  <backup>` and `  backed up  <second>`, the second folder has
  the file, and the exit code is 0

### CLI-005.3 – A backup conflict fails with exit code 1
- **Given** the backup folder has `2026/2026-10-17/DSCF0001.RAF` with other content
- **When** the user runs the command of CLI-005.1
- **Then** the destination has the file, the backup folder's file is unchanged, the
  output has `  backup failed  <backup>: name conflict`, and the exit code is 1

### CLI-005.4 – An invalid backup destination stops the command
- **Given** `--backup <destination>`, `--backup <source>/backup`, or the same
  `--backup` folder twice
- **When** the user runs the command
- **Then** standard error names the backup destination, no file or folder is created,
  and the exit code is 2

### CLI-005.5 – A dry run writes no backup
- **Given** the source of CLI-005.1 and a backup folder that does not exist
- **When** the user runs the command of CLI-005.1 with `--dry-run`
- **Then** it prints `would copy  DSCF0001.RAF -> 2026/2026-10-17/DSCF0001.RAF`, no
  backup line, the backup folder still does not exist, and the exit code is 0

## Out of scope

- Backup destinations in the guided import: CLI-003.
- Which files are backed up and how each copy is verified: IMPORT-007.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-04 | Initial draft for backup destinations (IMPORT-007); accepted by the owner |
| 2026-11-04 | Implemented in `framehive-cli` |
| 2026-11-04 | The import history recognizes a re-run (CLI-008): CLI-005.2 now runs without the history, so it still tests IMPORT-003 and IMPORT-007 |
