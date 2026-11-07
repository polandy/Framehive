# IMPORT-012 – Free space at the destinations

- Status: implemented
- Module: import
- Origin: I1
- Depends on: IMPORT-003, IMPORT-004, IMPORT-007, IMPORT-009

## Purpose

An import that runs out of space stops halfway, with a card that is partly imported and
partly backed up. The import therefore checks before the first copy that every
destination can hold the files it is about to write, and does not start when one
cannot (ADR 0033).

## Behaviour

**Bytes to write.** The plan decides, for each destination, which files are written
there. Their sizes add up to the bytes to write at that destination.

| Destination | Files that count |
|---|---|
| import destination | files the plan copies, also under a suffixed name (IMPORT-003) |
| backup destination | files that are backed up (IMPORT-007) and whose target path holds nothing there yet |

With a video destination (IMPORT-013), a video counts at the video destination and the
video backup destinations instead.

A file recognized as imported before (IMPORT-009), "already imported" (IMPORT-003),
skipped or failed by a name conflict, does not count at the import destination. A file
whose backup target already holds a file is "already backed up" or fails with a name
conflict; neither writes, so it does not count there.

**Per file system.** Destinations on the same file system share its free space. The
bytes to write at all of them are added up and compared with that file system's free
space. A destination that does not exist yet is on the file system of its nearest
existing parent folder.

**Not enough space.** When the bytes to write on a file system are more than its free
space, the run does not start, and nothing is created or copied at any destination. The
result names each destination on that file system, the bytes to write there in total,
and the free space. Every file system that is short is reported, not only the first.

**Free space unknown.** When the free space of a destination cannot be read, the run
goes on, and the result has a warning that names the destination and the reason.

**When.** The check runs after the plan and after the checks of the backup destinations
(IMPORT-007), before the first file is copied. A dry run checks too.

**Creating the destinations.** When the run starts, it checks the destinations
(IMPORT-007, IMPORT-013) and the free space first, and only then creates every missing
destination with its parents: the import destination, the video destination, the backup
destinations and the video backup destinations, also when the plan copies no file. A
folder that exists already, or that another program creates meanwhile, is used as it
is. A dry run creates nothing. A folder that cannot be created stops the run before any
sequence number is reserved, the start is recorded or a file is read; the result names
the destination and the folder that could not be created, with the reason. The folders
created before it stay.

## Acceptance criteria

### IMPORT-012.1 – An import that fits starts
- **Given** two planned files of 400 and 600 bytes, and an import destination with 1000
  bytes free
- **When** the import runs
- **Then** both files are copied

### IMPORT-012.2 – An import that does not fit does not start
- **Given** the files of IMPORT-012.1, and an import destination with 999 bytes free
  that does not exist yet
- **When** the import runs
- **Then** the result is "not enough space" with the import destination, 1000 bytes to
  write and 999 bytes free, the destination folder still does not exist, and no file is
  read

### IMPORT-012.3 – Destinations on one file system add up
- **Given** the files of IMPORT-012.1, and an import destination and a backup
  destination on the same file system with 1500 bytes free
- **When** the import runs
- **Then** the result is "not enough space" with both destinations, 2000 bytes to write
  and 1500 bytes free, and nothing is copied

### IMPORT-012.4 – Each file system is checked on its own
- **Given** the files of IMPORT-012.1, an import destination with 1000 bytes free, and
  a backup destination on another file system with 1000 bytes free
- **When** the import runs
- **Then** both files are copied and backed up

### IMPORT-012.5 – Files not written do not count
- **Given** three planned files of 500 bytes each: one recognized as imported before,
  one "already imported", and one new; and an import destination with 500 bytes free
- **When** the import runs
- **Then** the new file is copied

### IMPORT-012.6 – Files already at a backup target do not count there
- **Given** two files of 500 bytes, a backup destination with 500 bytes free whose
  target path of the first file holds an identical file, and an import destination on
  another file system with 1000 bytes free
- **When** the import runs
- **Then** both files are copied, the first is "already backed up" and the second
  backed up

### IMPORT-012.7 – Every short file system is reported
- **Given** the files of IMPORT-012.1, and an import destination and a backup
  destination on two file systems with 999 bytes free each
- **When** the import runs
- **Then** the result is "not enough space" with one entry for each destination

### IMPORT-012.8 – Unknown free space warns and goes on
- **Given** the files of IMPORT-012.1, and an import destination whose free space
  cannot be read
- **When** the import runs
- **Then** both files are copied, and the result has a warning with the destination
  and the reason

### IMPORT-012.9 – A dry run checks too
- **Given** the files and destination of IMPORT-012.2
- **When** a dry run runs
- **Then** the result is "not enough space" as in IMPORT-012.2

### IMPORT-012.10 – The destinations are created when the run starts
- **Given** a source with no file, an import destination that does not exist yet, and a
  backup destination `<dir>/new/backup` whose parents do not exist
- **When** the run starts
- **Then** the import destination and the backup destination exist, with the parents
  of the backup destination

### IMPORT-012.11 – A destination that cannot be created stops the run
- **Given** the files of IMPORT-012.1, and a backup destination `<file>/backup` below a
  file
- **When** the run starts
- **Then** the result is "destination not created" with the backup destination, the
  folder `<file>/backup` and the reason, and no file is copied

## Out of scope

- How the command line and the guided import report the shortage: CLI-011.
- A destination that fills up during the copy: IMPORT-004.
- Separate destinations for photos and videos (R7, I4): IMPORT-013; each then counts
  as a destination here.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-05 | Initial draft for I1 (ADR 0033); accepted by the owner |
| 2026-11-05 | Implemented in `files`, `adapter-free-space`, `import` and `framehive-cli`; `implemented` |
| 2026-11-07 | The run creates the missing destinations itself, after the free-space check; add IMPORT-012.10 and IMPORT-012.11 |
