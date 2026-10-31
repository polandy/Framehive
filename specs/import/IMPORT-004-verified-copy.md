# IMPORT-004 – Verified copy

- Status: accepted
- Module: import
- Origin: N1, N2, N7, N26
- Depends on: IMPORT-001

## Purpose

Photographers format their memory cards after an import, so a copy that is incomplete or
silently corrupt means a lost photo. Each imported file is therefore checked against its
source before the import reports it as copied, and no file at the destination is ever
half written or replaced.

## Behaviour

The import copies each file of the import plan from the source to its target path under
the import destination. It copies and never moves: nothing in the source is deleted,
renamed or written.

For each file:

1. The folders of the target path are created when they are missing.
2. The source is read once. While it is read, its checksum is computed and the bytes
   are written to `<target>.part` in the target folder.
3. The `.part` file gets the source's modification time, is written to the storage
   device (synced), then renamed to the target name, and the folder is synced. The
   rename never replaces a file.
4. The target is read back in full, and its checksum is compared with the source
   checksum.

Only then is the file reported as copied, with its checksum. Checksums use BLAKE3 and
are written `blake3:<hex>` (architecture §8).

A file that fails at any step is reported as failed, with the step and the reason. Its
`.part` file is removed, and a target that failed verification is removed too, because
the import wrote it. The import goes on with the next file. A file that exists at the
target name before the rename is never touched; whether the import plan may use that
name is decided by IMPORT-003, so a file found there at this point appeared during the
copy.

A `.part` file that is already at `<target>.part` is the remainder of an interrupted
import. It is replaced.

## Acceptance criteria

### IMPORT-004.1 – The copy matches the source
- **Given** a source file `DSCF0001.RAF` and its target path `2026/DSCF0001.RAF`
- **When** it is imported
- **Then** `2026/DSCF0001.RAF` has the same content as the source, and the file is
  reported as copied with the checksum `blake3:<hex>` of that content

### IMPORT-004.2 – The source is left unchanged
- **Given** a source file
- **When** it is imported
- **Then** the source file still exists, with the same content, name and modification
  time, and no other file is added to the source

### IMPORT-004.3 – Missing folders are created
- **Given** a destination without the folder `2026/2026-10-17`
- **When** a file with the target path `2026/2026-10-17/DSCF0001.RAF` is imported
- **Then** the folders are created and the file is copied

### IMPORT-004.4 – A read error leaves nothing behind
- **Given** a source file that cannot be read to its end
- **When** it is imported
- **Then** neither the target nor its `.part` file exists, and the file is reported as
  failed, with the reason

### IMPORT-004.5 – A write error leaves nothing behind
- **Given** a destination that fails while the `.part` file is written, for example
  because it is full
- **When** a file is imported
- **Then** neither the target nor its `.part` file exists, and the file is reported as
  failed, with the reason

### IMPORT-004.6 – A bad copy is detected and removed
- **Given** a destination that returns different bytes when the target is read back
- **When** a file is imported
- **Then** the target is removed, and the file is reported as failed verification,
  with both checksums

### IMPORT-004.7 – A copied file survives a crash
- **Given** a file reported as copied
- **When** the storage device loses every write that was not synced
- **Then** the target still exists with the full content

### IMPORT-004.8 – No partial file under the target name
- **Given** an import that stops at any point while a file is copied
- **When** the destination is inspected
- **Then** the target either does not exist or has the full, verified content; any
  partial content is only in `<target>.part`

### IMPORT-004.9 – A file that appears at the target is kept
- **Given** another program creates a file at the target path while the import copies
  to `<target>.part`
- **When** the import renames the `.part` file
- **Then** the other program's file is unchanged, the `.part` file is removed, and the
  file is reported as failed because the target exists

### IMPORT-004.10 – A leftover `.part` file is replaced
- **Given** `<target>.part` left by an interrupted import, and no file at the target
- **When** the file is imported again
- **Then** the target has the source's content, and `<target>.part` no longer exists

### IMPORT-004.11 – One failed file does not stop the import
- **Given** three files, and the second cannot be read
- **When** they are imported
- **Then** the first and third are copied and verified, and only the second is reported
  as failed

### IMPORT-004.12 – The target keeps the modification time
- **Given** a source file modified at 2026-10-17 14:05:09 UTC
- **When** it is imported
- **Then** the target's modification time is 2026-10-17 14:05:09 UTC

## Out of scope

- A target name that already exists, or two files with the same target, and files
  already imported: IMPORT-003.
- Which files are found in the source: IMPORT-005.
- Backup destinations (I2) and the "card can be formatted" status (N5): later specs.
- Resuming an interrupted import (N4), and the import log (N6): later specs.
- Moving files (G1): not planned for the import; a later spec if ever.
- How the command line reports results and exit codes: CLI-001.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-10-31 | Initial draft (milestone 1) |
| 2026-10-31 | The target keeps the source's modification time (IMPORT-004.12); accepted by the owner |
| 2026-10-31 | `copy_verified` in the `files` service covers IMPORT-004.1–.10 and .12 against `MemoryFileSystem`; stays `accepted` until `adapter-fs` runs it on a real file system and the import run covers IMPORT-004.11 |
