# CLI-007 – Format readiness in the command line

- Status: accepted
- Module: cli (`apps/cli`)
- Origin: N5, T7
- Depends on: CLI-001, CLI-003, CLI-005, CLI-006, IMPORT-008

## Purpose

After an import, the person wants one clear answer before formatting the memory card:
is every file saved at every destination? `framehive import` and the guided import
print the format readiness (IMPORT-008) as the last lines of their report.

## Behaviour

**Where.** `framehive import` (CLI-001) prints the format readiness on standard output
after the summary line and the backup summary lines (CLI-005). The guided import
(CLI-003, CLI-006) prints it after its report, before `Eject <name>? [Y/n]: `. A dry run
prints no format readiness.

**The lines.** `<n>` is the number of required locations; `location` is singular when
`<n>` is 1. `framehive import` says `source`, the guided import says `card`.

| Line | When |
|---|---|
| `The <card or source> can be formatted: every file is verified in <n> locations.` | The card can be formatted |
| `Do not format the <card or source>: not every file is verified in <n> locations.` | A file or folder keeps it from being formatted |
| `Do not format the <card or source>: the import was stopped.` | The run was stopped |
| `  <source path>: verified in <k> of <n> locations` | After the `Do not format` line, for each file that is not saved |
| `  <source path>: left out: <reason>` | After the `Do not format` line, for each file or folder left out for a reason that keeps the card |
| `Not saved: <m> left out by the scan.` | Last, when files were left out as unsupported, hidden or symbolic links |

Paths are relative to the source, as in CLI-001. `<reason>` is the left-out reason of
CLI-001.

**Unchanged.** The exit codes of CLI-001 and CLI-005 and the eject question of CLI-003
stay as they are: ejecting does not format the card, and a run with a skipped file
still exits with 0.

## Acceptance criteria

### CLI-007.1 – Every file saved
- **Given** the folders and the command of CLI-005.1
- **When** the user runs it
- **Then** the last line is
  `The source can be formatted: every file is verified in 2 locations.`, and the exit
  code is 0

### CLI-007.2 – Without backup destinations
- **Given** a source with `DSCF0001.RAF` and an empty destination
- **When** the user runs `framehive import <source> <destination>`
- **Then** the last line is
  `The source can be formatted: every file is verified in 1 location.`

### CLI-007.3 – A failed backup is listed
- **Given** the backup conflict of CLI-005.3
- **When** the user runs the command
- **Then** the output ends with
  `Do not format the source: not every file is verified in 2 locations.` and
  `  DSCF0001.RAF: verified in 1 of 2 locations`, and the exit code is 1

### CLI-007.4 – A skipped file is listed, and the exit code stays 0
- **Given** a name conflict under `--on-conflict skip`, as in CLI-001.4
- **When** the user runs the command
- **Then** the output ends with
  `Do not format the source: not every file is verified in 1 location.` and
  `  DSCF0001.RAF: verified in 0 of 1 location`, and the exit code is 0

### CLI-007.5 – An unreadable folder is listed
- **Given** a source with `DSCF0001.RAF` and a folder `locked` that cannot be read, as
  in CLI-001.13
- **When** the user runs the command
- **Then** the output ends with
  `Do not format the source: not every file is verified in 1 location.` and
  `  locked: left out: <reason>`

### CLI-007.6 – Camera system files are counted
- **Given** a source with `DSCF0001.RAF` and `notes.txt`
- **When** the user runs the command
- **Then** the output ends with
  `The source can be formatted: every file is verified in 1 location.` and
  `Not saved: 1 left out by the scan.`

### CLI-007.7 – A dry run prints no format readiness
- **Given** the source of CLI-005.1
- **When** the user runs the command with `--dry-run`
- **Then** the output has no line that starts with `The source can be formatted` or
  `Do not format`

### CLI-007.8 – The guided import shows it before the eject question
- **Given** a guided import of `EOS_DIGITAL` with one backup destination, in which every
  file is copied and backed up
- **When** the import finishes
- **Then** the dialog prints
  `The card can be formatted: every file is verified in 2 locations.` and then
  `Eject EOS_DIGITAL? [Y/n]: `

### CLI-007.9 – The guided import lists a file that is not saved
- **Given** a guided import of `EOS_DIGITAL` in which `DCIM/100CANON/IMG_0002.CR2`
  fails at the one backup destination
- **When** the import finishes
- **Then** the dialog prints
  `Do not format the card: not every file is verified in 2 locations.` and
  `  DCIM/100CANON/IMG_0002.CR2: verified in 1 of 2 locations`, and then asks
  `Eject EOS_DIGITAL? [Y/n]: `

### CLI-007.10 – A card pulled during the import
- **Given** the card pulled and inserted again of CLI-003.8
- **When** the import of the card finishes
- **Then** the dialog prints the format readiness once, for the whole card, and not when
  the card was pulled

## Out of scope

- Which files are saved and which keep the card: IMPORT-008.
- An eject or a prompt that depends on the format readiness: not planned; ejecting does
  not format the card.
- A machine-readable report of the import: a later spec.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-04 | Initial draft; owner decisions: no change to exit codes or to the eject question; accepted by the owner |
