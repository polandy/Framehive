# IMPORT-007 – Backup destinations

- Status: implemented
- Module: import
- Origin: I2, I5
- Depends on: IMPORT-003, IMPORT-004

## Purpose

A memory card is formatted after the import, so a single copy of a photo is one failed
disk away from loss. The import therefore writes each file to one or more backup
destinations as well, with the same folder structure and the same file names as in the
import destination, and verifies every copy.

## Behaviour

An import run has one import destination and zero or more backup destinations. The
import plan decides each file's target path in the import destination, including the
name conflicts of IMPORT-003. A backup destination never changes that decision: each
file goes to the same target path below every backup destination.

**Which files are backed up.** A file that is copied to the import destination, and a
file that is "already imported" there, is backed up. Backing up the files already
imported lets a re-run fill a new or a previously failed backup destination. A file
that is skipped or failed because of a name conflict (IMPORT-003) is not backed up.

**One read, verified copies.** The source is read once. While it is read, its checksum
is computed and the bytes are written to the `.part` file at the import destination and
at each backup destination. Each target is then synced, renamed, synced and read back
exactly as in IMPORT-004, and its checksum is compared with the source checksum. A file
already imported is read once in the same way and written to the backup destinations
only.

**Each destination on its own.** A failure at one destination removes only that
destination's `.part` file or unverified target (IMPORT-004) and does not stop or undo
the copy at the others. A backup destination that fails is reported for that file with
the step and the reason; the import goes on with the next file. A verified copy is never
removed because another destination failed.

**A file at the backup target.** A backup never overwrites a file.

| Found at the backup target | Result for that backup destination |
|---|---|
| nothing | the file is copied and verified: "backed up" |
| a file with identical content | the file is not written: "already backed up" |
| a file with other content | the file is not written: "failed: name conflict" |

**The result of a file** is its result at the import destination (IMPORT-003,
IMPORT-004), plus one result per backup destination. A file counts as copied when its
import-destination copy is verified, also when a backup destination failed. The summary
counts, for each backup destination, the files backed up, already backed up and failed.

**Before the run.** Backup destinations are checked before any file is copied. A backup
destination that is the import destination, that lies inside the import destination or
the source, that contains the import destination, or that is given twice, stops the run
before it starts. A missing backup destination is created with its parents, as the
import destination is.

## Acceptance criteria

### IMPORT-007.1 – A copy goes to every backup destination
- **Given** a source file `DSCF0001.RAF` with the target path `2026/DSCF0001.RAF`, and
  two backup destinations
- **When** it is imported
- **Then** the import destination and both backup destinations have
  `2026/DSCF0001.RAF` with the source's content and modification time, and the file is
  reported as copied and as backed up at both backup destinations

### IMPORT-007.2 – The backup keeps the resolved name
- **Given** the target path `DSCF0001.RAF` holds a file with other content in the import
  destination, the conflict policy `add_suffix`, and an empty backup destination
- **When** the file is imported
- **Then** both the import destination and the backup destination have
  `DSCF0001-1.RAF`, and the backup destination has no `DSCF0001.RAF`

### IMPORT-007.3 – The source is read once
- **Given** a source file and two backup destinations
- **When** it is imported
- **Then** the source file is read from start to end exactly once

### IMPORT-007.4 – Each backup is verified against the source
- **Given** a backup destination that returns different bytes when the target is read
  back
- **When** a file is imported
- **Then** the target at that backup destination is removed, and the file is reported
  as copied, and as failed verification at that backup destination with both checksums

### IMPORT-007.5 – A failed backup does not undo the import
- **Given** a backup destination that fails while the `.part` file is written
- **When** a file is imported
- **Then** the import destination has the verified file, the backup destination has
  neither the target nor its `.part` file, and the file is reported as copied and as
  failed at that backup destination, with the reason

### IMPORT-007.6 – A failed import destination keeps verified backups
- **Given** an import destination that fails while the `.part` file is written, and a
  backup destination that works
- **When** a file is imported
- **Then** the backup destination has the verified file, the import destination has
  neither the target nor its `.part` file, and the file is reported as failed and as
  backed up at the backup destination

### IMPORT-007.7 – A file already imported is backed up
- **Given** the import destination holds the file with identical content, and the
  backup destination is empty
- **When** the file is imported
- **Then** the file is marked "already imported", the import destination is unchanged,
  and the backup destination has the verified file, reported as backed up

### IMPORT-007.8 – Identical content at the backup target is kept
- **Given** the backup target holds a file with identical content
- **When** the file is imported
- **Then** the backup target is not written, and the file is reported as already backed
  up at that backup destination

### IMPORT-007.9 – Other content at the backup target is never overwritten
- **Given** the backup target holds a file with other content
- **When** the file is imported
- **Then** the backup target is unchanged, no `.part` file is left there, and the file
  is reported as failed at that backup destination because of a name conflict

### IMPORT-007.10 – A name conflict in the import destination is not backed up
- **Given** the target path holds a file with other content in the import destination,
  and the conflict policy `skip`
- **When** the file is imported
- **Then** it is marked "skipped: name conflict", and nothing is written to the backup
  destination

### IMPORT-007.11 – Invalid backup destinations stop the run before it starts
- **Given** a backup destination that is the import destination, that lies inside the
  import destination or the source, that contains the import destination, or that is
  given twice
- **When** the import run starts
- **Then** it does not start, nothing is written, and the reason names the backup
  destination

### IMPORT-007.12 – A missing backup destination is created
- **Given** a backup destination `<dir>/new/backup` whose parents do not exist
- **When** a file is imported
- **Then** the folders are created and the file is backed up there

### IMPORT-007.13 – The summary counts each backup destination
- **Given** three files and one backup destination, where the first is backed up, the
  second is already backed up and the third fails there
- **When** they are imported
- **Then** the summary for that backup destination counts 1 backed up, 1 already backed
  up and 1 failed

## Out of scope

- How the command line takes backup destinations and reports them: CLI-005; the guided
  import: CLI-003.
- The "card can be formatted" status once every file is verified in N locations (N5):
  IMPORT-008.
- Detecting backup drives by a marker folder, and fixed network paths (I3): a later spec.
- Separate backup destinations for photos and videos (I4), and separate destinations
  (R7): IMPORT-013.
- Overwriting a file at a backup target: not offered, because the import never
  overwrites (IMPORT-004).
- Enough free space at each destination (I1): IMPORT-012.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-04 | Initial draft; owner decisions: the source is read once and each copy is verified against its checksum, a backup never overwrites, a failed backup does not undo the import; accepted by the owner |
| 2026-11-04 | Implemented in `files` (`copy_verified_to_all`) and the import run; IMPORT-007.2 corrected to the suffix `-1` of IMPORT-003 |
