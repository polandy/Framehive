# CLI-009 – Import log in the command line

- Status: accepted
- Module: cli (`apps/cli`)
- Origin: N6
- Depends on: CLI-008, IMPORT-010

## Purpose

`framehive import-log` prints the import log (IMPORT-010): the list of import runs,
or which file of one run went where, with its checksum, as text to read or as JSON or
CSV to keep or process.

## Behaviour

**The command.** `framehive import-log [<run>] [--format text|json|csv]` reads the import
history in the state folder of CLI-008. It never creates the history or the state folder.
`<run>` is a run number, or `last` for the run with the highest number. `--format`
defaults to `text`; it applies only to the log of one run.

**The list of runs.** Without `<run>`, the command prints one line per run, oldest first:

```
<run>  <started>  <finished>  <copies> copies  <source> -> <import destination>
```

`<started>` and `<finished>` are the recorded times (RFC 3339). A run without an end
shows `not finished` in place of `<finished>`. Without any run, it prints
`No import runs recorded.`. `--format` with no `<run>` is a usage error.

**The log of one run as text.** With `<run>`, the command prints:

```
run <run>
started              <started>
finished             <finished>
source               <source>
import destination   <import destination>
video destination    <video destination>

<destination>
  <source path> -> <target path>  <checksum>
```

with one block per destination in the order of IMPORT-010, and `not finished` as above.
The line `video destination` is left out when the run has none (IMPORT-013).
A run without copies prints the header and `No copies recorded.`.

**JSON.** `--format json` prints one JSON object, followed by a newline:

```json
{"run": 3, "started_at": "…", "finished_at": null, "source": "…",
 "import_destination": "…", "video_destination": null, "copies": [{"destination": "…", "source_path": "…",
 "original_file_name": "…", "size": 10, "capture_time": "2026-10-17T14:05:09",
 "checksum": "blake3:…", "target": "…", "recorded_at": "…"}]}
```

`finished_at`, `video_destination` and `capture_time` are `null` when they are absent. The copies are in the
order of IMPORT-010.

**CSV.** `--format csv` prints RFC 4180 CSV with a header row and one row per copy,
in the order of IMPORT-010, lines ending in CRLF:

```
run,destination,source_path,original_file_name,size,capture_time,checksum,target,recorded_at
```

An absent capture time is an empty field. A field that holds a comma, a double quote or
a line break is quoted, and a double quote in it is doubled.

**Paths.** Paths are printed as on the lines of CLI-001; a path that is not valid UTF-8
has each invalid byte replaced by U+FFFD in every format.

**Exporting.** The log is printed to standard output; it is exported by redirecting it,
for example `framehive import-log 3 --format csv > import-3.csv`.

**Errors.** Each prints to standard error and exits with code 2:

| Case | Message |
|---|---|
| no run with that number | `error: no import run <run>` |
| `last` without any run | `error: no import runs recorded` |
| a history that cannot be used | `error: the import history <path> cannot be used: <reason>` |
| no state folder | `error: the home folder is unknown; set HOME` |

## Acceptance criteria

### CLI-009.1 – The list of runs
- **Given** two runs of `framehive import`, the second with a backup destination
- **When** `framehive import-log` runs
- **Then** the output has one line per run, the first run first, in the format above,
  and the exit code is 0

### CLI-009.2 – A run that is not finished
- **Given** a history with a run whose end is not recorded
- **When** `framehive import-log` runs
- **Then** its line shows `not finished`

### CLI-009.3 – No runs
- **Given** `XDG_STATE_HOME` set to an empty folder
- **When** `framehive import-log` runs
- **Then** the output is `No import runs recorded.`, the exit code is 0, and the folder
  is still empty

### CLI-009.4 – The log of one run as text
- **Given** the import of CLI-005.1 as run 1
- **When** `framehive import-log 1` runs
- **Then** the output has the header of run 1, the block of the import destination with
  `DSCF0001.RAF -> 2026/2026-10-17/DSCF0001.RAF  blake3:<hex>`, then the block of the
  backup destination with the same line, and the exit code is 0

### CLI-009.5 – The last run
- **Given** two runs
- **When** `framehive import-log last` runs
- **Then** the output is the log of the second run

### CLI-009.6 – JSON
- **Given** the import of CLI-001.1 as run 1
- **When** `framehive import-log 1 --format json` runs
- **Then** the output is one JSON object with the fields above and one copy, whose
  `checksum` starts with `blake3:` and whose `finished_at` is not `null`

### CLI-009.7 – JSON for a file without a capture time
- **Given** a run that copied `notes.png`, which records no capture time
- **When** `framehive import-log 1 --format json` runs
- **Then** its copy has `"capture_time": null`

### CLI-009.8 – CSV
- **Given** the import of CLI-005.1 as run 1
- **When** `framehive import-log 1 --format csv` runs
- **Then** the output is the header row and two rows, one for the import destination
  and one for the backup destination, each ending in CRLF

### CLI-009.9 – CSV quoting
- **Given** a run that copied `a,"b".jpg`
- **When** `framehive import-log 1 --format csv` runs
- **Then** its file name field is `"a,""b"".jpg"`

### CLI-009.10 – A run that does not exist
- **Given** a history without run 7
- **When** `framehive import-log 7` runs
- **Then** standard error has `error: no import run 7`, and the exit code is 2

### CLI-009.11 – A history that cannot be used
- **Given** an `import-history.sqlite` in the state folder that is not a SQLite database
- **When** `framehive import-log` runs
- **Then** standard error has `error: the import history <path> cannot be used: …`, the
  file is unchanged, and the exit code is 2

### CLI-009.12 – `--format` without a run
- **Given** any history
- **When** `framehive import-log --format csv` runs
- **Then** it is a usage error with exit code 2

### CLI-009.13 – A run with a video destination
- **Given** the import of CLI-012.1 as run 1
- **When** `framehive import-log 1` runs, and again with `--format json`
- **Then** the text has the line `video destination    <videos>` after the import
  destination and the block of `<videos>` with `clip.MOV`, and the JSON has
  `"video_destination": "<videos>"`

## Out of scope

- What the log holds: IMPORT-010.
- An option to write the export to a file: redirect standard output.
- Filtering the list by date, source or destination: not planned until it is needed.
- Exporting several runs at once: not planned until it is needed.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-04 | Initial draft; owner decisions: `framehive import-log`, runs oldest first, `--format` only for one run, paths that are not valid UTF-8 printed with U+FFFD, `framehive import` does not print its run number; accepted by the owner |
| 2026-11-04 | Implemented in `framehive-cli`; `implemented` |
| 2026-11-06 | The video destination of a run (CLI-012); accepted by the owner |
