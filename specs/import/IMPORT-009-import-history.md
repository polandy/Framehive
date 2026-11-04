# IMPORT-009 – Import history

- Status: accepted
- Module: import
- Origin: N3, N4, T1, T8
- Depends on: IMPORT-002, IMPORT-003, IMPORT-004, IMPORT-007, IMPORT-008

## Purpose

A card imported a second time must not fill the library with copies, also with a
persisted sequence counter, after photos were renamed or moved in the library, or after
an interrupted import. The import
therefore keeps a history of every verified copy and recognizes a file by its content
before it is copied again.

## Behaviour

The import history is machine-local and survives restarts (ADR 0031). A run uses it
for the import destination and for each backup destination; each destination is
identified by its canonical absolute path.

**What is recorded.** Every copy that is verified at a destination is recorded with the
destination, the source path, the original file name, the size, the capture time as the
file records it (before camera time correction), the checksum and the target path. A
copy is recorded as soon as it is verified, before the next file is read.

| Result at a destination | Recorded |
|---|---|
| copied (IMPORT-004), already imported (IMPORT-003) | yes, at the import destination |
| backed up, already backed up (IMPORT-007) | yes, at that backup destination |
| imported before, backed up before | no, it is recorded already |
| duplicate, skipped, failed | no |

A run's start and end are recorded too; a run that the progress sink stopped
(IMPORT-004.13) has an end, a crashed one has none. A dry run records nothing.

**Imported before.** When the plan is made, each file the scan found is looked up at the
import destination. A file is *imported before* when the history holds a copy at the
import destination with the same original file name and size, the same capture time
when the file records one, and the same checksum. The checksum is computed only for a
file that matches name, size and capture time, so a card without matches is not
read before the run. A file imported before:

- is not named by the template and gets no sequence number (IMPORT-002);
- is not copied, and the import destination is not looked at again: a file renamed,
  moved or deleted in the library stays imported before;
- is reported at the target path of its most recent recorded copy.

Any other file is planned as before, including the name conflicts of
IMPORT-003. Recognition does not depend on the name template: changing the template does
not make a file new.

**Backed up before.** A file is *backed up before* at a backup destination when the
history holds a copy there with the same name, size, capture time and checksum; nothing
is written there. A file imported before that is not backed up
before at a backup destination is backed up from the source to its reported target path
there, by the rules of IMPORT-007. So a re-run completes a backup that failed or was added
later.

**Resuming.** An interrupted import (stopped, failed or killed) is resumed by running it
again: files verified before are imported before or backed up before, and only the rest
is copied. Sequence numbers reserved by the interrupted run
and not used stay unused (ADR 0029).

**Format readiness.** A file imported before is verified at the import destination, and
a file backed up before at that backup destination (IMPORT-008).

**A history that cannot be used.** A missing history starts empty. A history that cannot
be opened, read or written, or has an unknown layout, stops the run before any file is
copied, and the history is not changed (ADR 0031, section 6). A history that cannot record
a copy later stops the run after that file.

## Acceptance criteria

### IMPORT-009.1 – A verified copy is recorded
- **Given** `DSCF0001.RAF`, 10 bytes, captured at 2026-10-17 14:05:09, and an empty
  history
- **When** it is imported to `2026-10-17/DSCF0001.RAF`
- **Then** the history holds one copy at the import destination with the file name, the
  size, the capture time, the checksum and that target path

### IMPORT-009.2 – A persisted counter does not copy a file again
- **Given** the import of IMPORT-009.1 with the template `{seq_stored:4}.{ext}`, which
  imported the file as `0001.RAF`
- **When** the same source is imported again with the same template
- **Then** the file is imported before at `0001.RAF`, nothing is copied, and no sequence
  number is reserved

### IMPORT-009.3 – A file renamed or deleted in the library is still recognized
- **Given** the import of IMPORT-009.1, after which the target was renamed, or deleted
- **When** the same source is imported again
- **Then** the file is imported before at `2026-10-17/DSCF0001.RAF`, and nothing is
  copied

### IMPORT-009.4 – Same name and size, other content
- **Given** a recorded copy of `DSCF0001.RAF` with 10 bytes, and a new `DSCF0001.RAF`
  with the same size and capture time but other content
- **When** it is imported
- **Then** it is not imported before; it is planned and copied by IMPORT-003

### IMPORT-009.5 – A card without matches is not read before the run
- **Given** a history whose copies differ from every file of the source in name, size or
  capture time
- **When** the plan is made
- **Then** no file of the source is read to compute a checksum

### IMPORT-009.6 – A file without a capture time
- **Given** a recorded copy of `notes.png`, which records no capture time, and the same
  `notes.png` on the source with a new modification time
- **When** the same source is imported again
- **Then** the file is imported before, matched by its name, size and checksum

### IMPORT-009.7 – Another import destination
- **Given** the import of IMPORT-009.1 into one import destination
- **When** the same source is imported into another, empty import destination
- **Then** the file is copied there and recorded at that import destination too

### IMPORT-009.8 – A changed template does not make a file new
- **Given** the import of IMPORT-009.1 with `{date:%Y-%m-%d}/{name}.{ext}`
- **When** the same source is imported again with `{name}.{ext}`
- **Then** the file is imported before at `2026-10-17/DSCF0001.RAF`

### IMPORT-009.9 – Resuming an interrupted import
- **Given** three files and the template `{seq_stored:4}.{ext}`, and a first run that
  the progress sink stopped after the first file, as `0001`
- **When** the same source is imported again
- **Then** the first file is imported before at `0001.RAF`, the second and third are
  copied as `0004` and `0005`, and the numbers 2 and 3 stay unused

### IMPORT-009.10 – A re-run completes a backup
- **Given** a file recorded at the import destination at `2026-10-17/DSCF0001.RAF` and not
  at the backup destination, whose backup had failed
- **When** the same source is imported again with that backup destination
- **Then** the file is imported before, and backed up to `2026-10-17/DSCF0001.RAF` at
  the backup destination and recorded there

### IMPORT-009.11 – Backed up before
- **Given** a file recorded at the import destination and at the backup destination, and
  deleted from the backup destination since
- **When** the same source is imported again with that backup destination
- **Then** the file is imported before and backed up before, and nothing is written

### IMPORT-009.12 – A file already imported is recorded
- **Given** an import destination that holds the content of `DSCF0001.RAF` at its target
  path, and an empty history
- **When** the source is imported
- **Then** the file is already imported (IMPORT-003) and recorded, and after the target
  is renamed a re-run finds it imported before

### IMPORT-009.13 – Files that are not verified are not recorded
- **Given** a run with a duplicate, a file skipped by a name conflict, a file whose copy
  fails, and a backup destination that fails for a fourth file
- **When** it is imported
- **Then** the history holds no copy for the first three, and for the fourth only its copy
  at the import destination

### IMPORT-009.14 – A dry run reads the history and records nothing
- **Given** the import of IMPORT-009.1
- **When** the same source and a new file are imported with a dry run
- **Then** the first file is imported before, the new one would be copied, and the history
  is unchanged

### IMPORT-009.15 – A history that cannot be used
- **Given** a history that cannot be read
- **When** an import starts
- **Then** the run does not start, no file is copied, and the error says that the history
  cannot be used

### IMPORT-009.16 – A history that cannot record a copy
- **Given** three files, and a history that fails when the second file's copy is recorded
- **When** they are imported
- **Then** the first two files are copied, the run stops after the second, and the third
  is not copied

### IMPORT-009.17 – Format readiness counts recorded copies
- **Given** a re-run with one backup destination in which the only file is imported
  before and backed up before
- **When** the format readiness is evaluated
- **Then** the card can be formatted, with 2 required locations

### IMPORT-009.18 – A run's start and end are recorded
- **Given** an import that the progress sink stops after the first file
- **When** it is imported
- **Then** the history holds the run with its source, its import destination, a start
  and an end

## Out of scope

- How the history is stored, its location and its layout: ADR 0031.
- How the command line reports them: CLI-008.
- Exporting the history as an import log (N6): a later spec.
- Files recognized after a camera time correction (N15) is set: the lookup uses the
  capture time before correction; tested with the camera time correction spec.
- Importing a file imported before again on request: not planned yet.
- Sharing the history between machines: not planned (ADR 0003, ADR 0031).
- Pairing RAW and JPEG files across separate import jobs through the history:
  LIBRARY-001.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-04 | Initial draft; owner decisions: SQLite in the state folder, name, size and capture time first and the checksum last, files deleted from the library stay imported before and count as saved, resuming is running the import again, no override yet; accepted by the owner |
