# IMPORT-010 – Import log

- Status: implemented
- Module: import
- Origin: N6
- Depends on: IMPORT-009, IMPORT-013

## Purpose

After an import, a person must be able to see which file of the source went where,
with its checksum, also weeks later and without the source at hand. The import log
shows this for each import run, read from the import history (IMPORT-009, ADR 0031).

## Behaviour

The import log is a view of the import history. It never writes the history and never
creates it.

**Runs.** The log lists every recorded import run, oldest first, with its run number,
its start, its end, its source, its import destination, its video destination when it
has one (IMPORT-013), and the number of its copies. A
run without an end (it is still running, or it crashed) is shown as not finished. The
run number is the number the history gave the run; it never changes and is never
reused.

**The log of one run.** For one run, the log shows the run as above and every copy the
run recorded, grouped by destination: the import destination first, then the video destination when
the run has one, then each backup destination in the order the run first recorded a copy there. Within a destination, the
copies are in the order they were recorded. Each copy shows what IMPORT-009 records:
the source path, the original file name, the size, the capture time when the file
records one, the checksum, the target path and when it was recorded.

**What the log does not show.** The history records only verified copies (IMPORT-009),
so the log of a run does not list files that were imported before, backed up before,
duplicates, skipped, failed or left out; those are reported by the run itself. A file
imported before appears in the log of the run that copied it. A dry run records nothing
and therefore has no log.

**A history that cannot be used.** A missing history has no runs; it is not created. A
history that cannot be opened or read, or has an unknown layout, is an error that
names the file; the history is not changed.

## Acceptance criteria

### IMPORT-010.1 – Runs are listed oldest first
- **Given** a history with two runs, the second with one backup destination
- **When** the runs are listed
- **Then** both are listed, the first run first, each with its run number, start, end,
  source, import destination and number of copies, counting the copies at every
  destination

### IMPORT-010.2 – A run without an end
- **Given** a run whose start is recorded and whose end is not
- **When** the runs are listed
- **Then** the run is listed as not finished

### IMPORT-010.3 – The copies of one run
- **Given** a run that recorded `DSCF0001.RAF` and `DSCF0002.RAF` at the import
  destination and `DSCF0001.RAF` at a backup destination
- **When** the log of that run is read
- **Then** it holds the two copies at the import destination in the order they were
  recorded, then the copy at the backup destination, each with source path, original
  file name, size, capture time, checksum, target path and recording time

### IMPORT-010.4 – A file without a capture time
- **Given** a run that recorded `notes.png`, which records no capture time
- **When** the log of that run is read
- **Then** its copy has no capture time

### IMPORT-010.5 – Only the run's own copies
- **Given** two runs of the same source, the second of which found every file imported
  before
- **When** the log of the second run is read
- **Then** it holds no copy, and the log of the first run holds them all

### IMPORT-010.6 – A run that does not exist
- **Given** a history without run 7
- **When** the log of run 7 is read
- **Then** the result says that there is no such run

### IMPORT-010.7 – A missing history
- **Given** no history file
- **When** the runs are listed
- **Then** the list is empty, and no history file is created

### IMPORT-010.8 – A history that cannot be used
- **Given** a history file that is not a SQLite database
- **When** the runs are listed
- **Then** the result is an error that names the file, and the file is unchanged

### IMPORT-010.9 – A run with a video destination
- **Given** a run that copied `DSCF0001.RAF` to the import destination, `clip.MOV` to
  a video destination, and both to one backup destination
- **When** the log of the run is read
- **Then** the run has that video destination, and the copies are grouped as the import
  destination, the video destination, then the backup destination; a run recorded
  before video destinations existed has none

## Out of scope

- What the history records and when: IMPORT-009.
- How the command line prints and exports the log: CLI-009.
- The log in the guided import: not planned until it is needed.
- Deleting runs from the history: not planned.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-04 | Initial draft; the history is read without being created or changed; accepted by the owner |
| 2026-11-04 | Implemented in `history` (`ImportHistory::list_runs`, `ImportHistory::read_run_log`) and `adapter-sqlite` (`SqliteImportHistory::read_only`); `implemented` |
| 2026-11-06 | The video destination of a run (IMPORT-013); accepted by the owner |
