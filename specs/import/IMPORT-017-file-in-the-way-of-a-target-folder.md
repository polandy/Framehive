# IMPORT-017 – A file in the way of a target folder

- Status: implemented
- Module: import
- Origin: I5, R7
- Depends on: IMPORT-003, IMPORT-012, IMPORT-013

## Purpose

A file can sit where a folder of a target path should be. The copy cannot create the
folder there, so the file fails. The dry run must show that failure, and the run must
name the file in the way, not a folder that "does not exist".

## Behaviour

When the import plan is made, every folder of a file's target path is matched at the
destination by its canonical caseless key, as IMPORT-003 matches the target folders.

**A file in the way.** When a folder of the target path is found at the destination
only as a file, with no folder of the same key, the file is not copied and is marked
as failed: "the target folder <path> is a file", where `<path>` is the file in the way,
spelled as found there. This holds for every conflict policy: a suffix changes only
the name, so no policy frees the folder. A dry run reports the same. Such a file
writes nothing, so it does not count toward the free space (IMPORT-012).

**The folders of a video destination.** A video destination that lies inside the
import destination is a folder the run needs (IMPORT-013). Its path below the import
destination, and each folder on that path, count as taken for the photos of the plan,
like a folder at the target path (IMPORT-003.7): the conflict policy decides. So a
photo never takes the name of a folder the videos need, and a dry run, which creates
no folder, plans the same names as the run, which created the video destination
before it starts.

## Acceptance criteria

### IMPORT-017.1 – A file where a target folder should be
- **Given** the destination holds a file `2026`, and a file of the plan has the target
  `2026/DSCF0001.RAF`
- **When** the import plan is made with any conflict policy
- **Then** the file is marked as failed with "the target folder `<destination>/2026` is
  a file"

### IMPORT-017.2 – A file deeper in the target path
- **Given** the destination holds a folder `2026` with a file `10` in it, and the target
  `2026/10/DSCF0001.RAF`
- **When** the import plan is made
- **Then** the file is marked as failed, naming `<destination>/2026/10`

### IMPORT-017.3 – The file in the way, in another spelling
- **Given** the destination holds a file `CAFÉ`, and the target `Café/DSCF0001.JPG`
- **When** the import plan is made
- **Then** the file is marked as failed, naming `<destination>/CAFÉ` as found there

### IMPORT-017.4 – The dry run and the run report it alike
- **Given** the destination holds a file where a target folder should be
- **When** the source is imported once as a dry run and once for real
- **Then** both mark the file as failed with the same reason, and the run writes
  nothing at the destination

### IMPORT-017.5 – A photo at a folder of the video destination
- **Given** a video destination `<import destination>/DSCF0001.JPG` that does not
  exist, a photo `DSCF0001.JPG`, the template `{name}.{ext}`, and the policy
  `add_suffix`
- **When** the source is imported once as a dry run and once for real
- **Then** both give the photo the target `DSCF0001-1.JPG`, and with the policy
  `skip` the photo is marked "skipped: name conflict"

### IMPORT-017.6 – A file in the way needs no free space
- **Given** the destination holds a file where the only photo's target folder should
  be, and 1 byte free at the import destination
- **When** the import run starts
- **Then** it is not stopped for lack of space, and the photo fails as in IMPORT-017.1

## Out of scope

- A file at the video destination, or on its path: the video destination is created
  before the run (IMPORT-013, "Before the run").
- A folder at the target path itself: IMPORT-003.7.
- Target names that another import job reserved as files: IMPORT-015.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-08 | Created and implemented from the open-work list (IMPORT-003, IMPORT-013): the dry run said it would copy, and the run failed the file with "<folder> does not exist" |
