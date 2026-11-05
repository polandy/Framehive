# IMPORT-008 – Format readiness

- Status: implemented
- Module: import
- Origin: N5, T7
- Depends on: IMPORT-003, IMPORT-004, IMPORT-005, IMPORT-007

## Purpose

A memory card is formatted after the import, and formatting it too early loses every
photo that is not yet saved elsewhere. After an import run, the import therefore tells
whether every file of the source is verified at every destination, so the person knows
when the card can be formatted, and which files keep it from being formatted.

## Behaviour

The format readiness of a finished import run is evaluated from the run's results
alone: it reads no file and does not look at the destinations again.

**Required locations.** The import destination and each backup destination of the run
are the required locations. A file is *saved* when it is verified at every one of them.
Without backup destinations, the import destination is the only required location.

**Verified at a location.**

| Result | Verified at |
|---|---|
| copied (IMPORT-004) | the import destination |
| already imported (IMPORT-003) | the import destination |
| backed up, already backed up (IMPORT-007) | that backup destination |
| skipped or failed, at any destination | not verified there |

A duplicate (an earlier file of the run with the same target and content, IMPORT-003)
is verified wherever that earlier file is verified.

**Files left out by the scan** (IMPORT-005) are on the source but are not imported.

| Reason | Effect |
|---|---|
| cannot be read | keeps the card from being formatted |
| sidecar, needs grouping | keeps the card from being formatted |
| unsupported type, hidden, symbolic link | counted as left out and not saved; does not keep the card from being formatted |

Cameras write their own system files, which the scan leaves out as unsupported or
hidden; they would otherwise keep every card from being formatted.

**Result.** The card *can be formatted* when every file the scan found is saved, no
file or folder was left out for a reason that keeps it from being formatted, and the
run finished. Otherwise the result lists each file that keeps it from being formatted,
in scan order, with the number of required locations it is verified at, or with its
left-out reason. Both results carry the number of required locations and the number of
files left out and not saved.

A run that the progress sink stopped (IMPORT-004.13) and a dry run never let the card be
formatted: a stopped run has not looked at every file, and a dry run verifies nothing.

## Acceptance criteria

### IMPORT-008.1 – Every file copied and backed up
- **Given** two files and one backup destination, both copied and backed up
- **When** the format readiness is evaluated
- **Then** the card can be formatted, with 2 required locations

### IMPORT-008.2 – Files already imported and already backed up are saved
- **Given** a re-run in which one file is already imported and already backed up
- **When** the format readiness is evaluated
- **Then** the card can be formatted

### IMPORT-008.3 – Without backup destinations, one location is required
- **Given** one copied file and no backup destination
- **When** the format readiness is evaluated
- **Then** the card can be formatted, with 1 required location

### IMPORT-008.4 – A failed backup keeps the card
- **Given** two backup destinations, and a file copied and backed up at the first one
  and failed at the second
- **When** the format readiness is evaluated
- **Then** the card cannot be formatted, and the file is listed as verified at 2 of 3
  locations

### IMPORT-008.5 – A failed import destination keeps the card
- **Given** a file that failed at the import destination and was backed up at the one
  backup destination
- **When** the format readiness is evaluated
- **Then** the card cannot be formatted, and the file is listed as verified at 1 of 2
  locations

### IMPORT-008.6 – A skipped file keeps the card
- **Given** a file skipped because of a name conflict
- **When** the format readiness is evaluated
- **Then** the card cannot be formatted, and the file is listed as verified at 0 of 1
  locations

### IMPORT-008.7 – A duplicate counts as its earlier file
- **Given** a file that is a duplicate of an earlier file, once when the earlier file is
  copied and once when it failed
- **When** the format readiness is evaluated
- **Then** the card can be formatted in the first case; in the second, both files are
  listed as verified at 0 of 1 locations

### IMPORT-008.8 – An unreadable file or folder keeps the card
- **Given** a run in which every file is copied, and a folder the scan could not read
- **When** the format readiness is evaluated
- **Then** the card cannot be formatted, and the folder is listed with its left-out
  reason

### IMPORT-008.9 – A sidecar left out keeps the card
- **Given** a run in which every file is copied, and a sidecar left out because it needs
  grouping
- **When** the format readiness is evaluated
- **Then** the card cannot be formatted, and the sidecar is listed with its left-out
  reason

### IMPORT-008.10 – Camera system files do not keep the card
- **Given** a run in which every file is copied, and a file left out as an unsupported
  type, a hidden file and a symbolic link
- **When** the format readiness is evaluated
- **Then** the card can be formatted, with 3 files left out and not saved

### IMPORT-008.11 – A stopped run keeps the card
- **Given** a run of two files that was stopped after the first file was copied
- **When** the format readiness is evaluated
- **Then** the card cannot be formatted, and the result says that the run was stopped

### IMPORT-008.12 – A dry run keeps the card
- **Given** a dry run of one file
- **When** the format readiness is evaluated
- **Then** the card cannot be formatted

### IMPORT-008.13 – An empty source can be formatted
- **Given** a source in which the scan found no file and left nothing out
- **When** the format readiness is evaluated
- **Then** the card can be formatted

## Out of scope

- How the command line and the guided import show the format readiness: CLI-007.
- A configurable number of required locations: a later version of the import profile
  (IMPORT-011).
- Formatting the card, or deleting the imported files from it (move mode, G1).
- Checking the destinations again later, for example after a reboot: the result holds
  for the run that produced it.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-04 | Initial draft; owner decisions: every destination of the run is required, files left out as unsupported, hidden or symbolic links do not keep the card, no change to exit codes; accepted by the owner |
