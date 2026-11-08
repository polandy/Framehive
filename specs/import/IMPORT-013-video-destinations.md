# IMPORT-013 – Video destinations

- Status: implemented
- Module: import
- Origin: R7, I4
- Depends on: IMPORT-003, IMPORT-007, IMPORT-009, IMPORT-012

## Purpose

Many people keep videos apart from photos, in another folder or on another disk, and
back them up elsewhere too. An import run can therefore send its videos to a video
destination and to video backup destinations of their own, in one run with the photos.

## Behaviour

**Which files are videos.** A file is a video when the scan gives it the role `Video`
(IMPORT-005). Every other file that is imported (`Raw`, `Rendered`) is a photo.

**The destinations of a file.** An import run has, besides its import destination and
backup destinations (IMPORT-007), an optional video destination and an optional list of
video backup destinations. A setting for videos that is not given takes the setting for
photos:

| Setting for videos | When not given, videos go to |
|---|---|
| video destination | the import destination |
| video backup destinations | the backup destinations |

A photo goes to the import destination and the backup destinations, as before. A video
goes to the video destination and the video backup destinations. Videos without any
backup, while photos have one, are not offered. Everything IMPORT-003, IMPORT-004 and
IMPORT-007 say about "the import destination" and "the backup destinations" of a file
holds for the destinations of that file.

**One plan.** The name template, the conflict policy, the job code and the sequence
numbers are the same for photos and videos: the plan decides one target path per file
(IMPORT-001, IMPORT-002) and then places it below that file's destinations. A name
conflict is a conflict at the file's own destination. A photo and a video never get
the same file, even when the video destination lies inside the import destination:
every token value is free of `/` (IMPORT-001, IMPORT-006), so one template gives every
target the same number of segments, and a video's path below the import destination
has the segments of the video destination on top. A name template of its own for
videos (see "Out of scope") would have to check that again.

**Imported before.** The import history (IMPORT-009) is searched at the file's own
destination. A video imported to the import destination by an earlier run is therefore
not recognized as imported before when a later run has a video destination; it is
copied there again.

**Free space.** A file counts at its own destinations (IMPORT-012); a video destination
and the import destination on the same file system share its free space.

**Before the run.** These checks join those of IMPORT-007, "Before the run", and stop the
run before it starts, with a reason that names the folder:

- a video destination that lies inside the source, or contains the source;
- a video backup destination that is or lies inside or contains the video destination,
  or the import destination, or lies inside the source; a backup destination is checked
  against the video destination in the same way;
- a folder given twice in the video backup destinations.

A video destination may be the import destination or lie inside it. A folder may be
both a backup destination and a video backup destination. A missing video destination
or video backup destination is created with its parents, once the checks and the
free-space check pass (IMPORT-012).

**The record of the run.** The import history records the video destination of a run,
or none (ADR 0031, addendum of 2026-11-06). Each copy keeps its own destination as
before, so the import log (IMPORT-010) lists videos under their own folders.

## Acceptance criteria

### IMPORT-013.1 – Videos go to the video destination
- **Given** a source with `DSCF0001.RAF` and `clip.MOV`, and a video destination
- **When** they are imported
- **Then** the import destination has the target of `DSCF0001.RAF` and not that of
  `clip.MOV`, and the video destination has the target of `clip.MOV` and not that of
  `DSCF0001.RAF`

### IMPORT-013.2 – Without a video destination videos stay with photos
- **Given** the source of IMPORT-013.1 and no video destination
- **When** they are imported
- **Then** both targets are in the import destination

### IMPORT-013.3 – Video backup destinations
- **Given** the source of IMPORT-013.1, a backup destination and a video backup
  destination
- **When** they are imported
- **Then** the backup destination has only the target of `DSCF0001.RAF`, the video
  backup destination has only the target of `clip.MOV`, and both are reported as backed
  up there

### IMPORT-013.4 – Without video backup destinations videos use the backup destinations
- **Given** the source of IMPORT-013.1, a video destination, a backup destination and
  no video backup destinations
- **When** they are imported
- **Then** the backup destination has the targets of both files

### IMPORT-013.5 – A name conflict at the video destination
- **Given** the target path of `clip.MOV` holds a file with other content in the video
  destination but not in the import destination, and the policy `add_suffix`
- **When** it is imported
- **Then** the video destination has the target with the suffix `-1`, and the import
  destination has no file of that name

### IMPORT-013.6 – Imported before at the video destination
- **Given** a history that records `clip.MOV` at the video destination, and the target
  is still there
- **When** the source is imported again with the same video destination
- **Then** `clip.MOV` is recognized as imported before and nothing is written

### IMPORT-013.7 – Free space counts videos at the video destination
- **Given** a video destination on a file system whose free space is smaller than the
  size of `clip.MOV`, and an import destination with enough free space on another one
- **When** the run starts
- **Then** it does not start, and the result names the video destination, the bytes to
  write there and its free space

### IMPORT-013.8 – Invalid video destinations stop the run before it starts
- **Given** a video destination inside the source; a video backup destination that is
  the video destination, lies inside it, or lies inside the import destination; a
  backup destination inside the video destination; or a video backup destination given
  twice
- **When** the import run starts
- **Then** it does not start, nothing is written, and the reason names the folder

### IMPORT-013.9 – A video destination inside the import destination
- **Given** a video destination `<import destination>/Videos` that does not exist
- **When** the source of IMPORT-013.1 is imported
- **Then** the folder is created, it has the target of `clip.MOV`, and the import
  destination has the target of `DSCF0001.RAF`

### IMPORT-013.10 – The run records its video destination
- **Given** a run with a video destination, and a run without one
- **When** the import history lists the runs
- **Then** the first has that video destination and the second has none

## Out of scope

- A name template of its own for videos: a later spec.
- Video metadata tokens such as codec or length (R3): a later spec.
- Video destinations in the guided import (CLI-002): it does not use import profiles
  yet, so it sends videos to its import destination as before.
- How the command line and an import profile give the settings: CLI-012, IMPORT-011.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-06 | Initial draft; owner decisions: separate destinations only, a setting for videos that is not given takes the setting for photos, a video destination may lie inside the import destination, shared sequence counters; accepted by the owner |
| 2026-11-07 | Missing destinations are created once the free-space check passes (IMPORT-012) |
| 2026-11-07 | Why photos and videos never get the same file at nested destinations |
