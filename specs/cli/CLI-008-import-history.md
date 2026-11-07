# CLI-008 – Import history in the command line

- Status: implemented
- Module: cli (`apps/cli`)
- Origin: N3, N4
- Depends on: CLI-001, CLI-003, CLI-004, CLI-005, CLI-006, IMPORT-009

## Purpose

`framehive import` and the guided import use the import history (IMPORT-009), so the
person sees which files were imported before and are not copied again, and can resume an
interrupted import by running it again.

## Behaviour

**Where the history is.** Both commands keep the history in `import-history.sqlite` in
the state folder of CLI-004: `$XDG_STATE_HOME/framehive/` when `XDG_STATE_HOME` is set
and absolute, otherwise `~/.local/state/framehive/` (ADR 0031). The folder is created
when it is missing. `framehive import` always needs the state folder, also with
`--dry-run` and with a template without a persisted counter: when `XDG_STATE_HOME` is
not an absolute path and the home folder is unknown, it prints
`error: the home folder is unknown; set HOME` to standard error, copies nothing, and
exits with code 2. A dry run reads an existing history and leaves the state folder
as it was: it records nothing and leaves no journal file of SQLite behind.

**The lines.** A file imported before gets its own line on standard output, in the
place of the lines of CLI-001, and a backup destination it is backed up before at gets
its own line below it, in the place of the lines of CLI-005:

| Line | Meaning |
|---|---|
| `imported before  <source path> -> <target path>` | Recorded at the import destination (IMPORT-009); not copied |
| `  backed up before  <backup>` | Recorded at that backup destination; nothing written |

With `--dry-run`, and in the file list of the guided import (CLI-003.2), the lines read
the same. The template preview of the guided import (CLI-002) shows a file imported
before as `  imported before  <source path> -> <target path>`. The summary of CLI-001 counts files imported before as `already imported`,
and the backup summary of CLI-005 counts files backed up before as `already backed up`:
in both cases the file is in the library and nothing is copied. Exit codes count them as
they count those.

**No more warning.** The warning of CLI-004 and the sentence in the plan summary of
CLI-003 that files already imported are not recognized with a persisted counter are no
longer printed: the history recognizes them.

**A history that cannot be used.** `framehive import` prints
`error: the import history <path> cannot be used: <reason>` to standard error, copies
nothing, and exits with code 2. The guided import prints
`The import history <path> cannot be used: <reason>` and does not import the card. A
history that fails while the import runs stops it after the current file; the command
reports the files done so far, prints the error as above, and exits with code 1.

**A folder that cannot be resolved.** The history identifies the import destination and
each backup destination by its canonical absolute path (IMPORT-009). When the guided
import cannot resolve one of them, it prints `Could not use <folder>: <reason>` and does
not import the card. When it cannot resolve the import destination for the template
preview, it prints `The template cannot be previewed: <reason>` and asks whether to use
the template without a preview.

**Resuming.** An import that was interrupted, by a crash, a pulled card or a cancelled
command, is resumed by running the same command again, or by inserting the card again in
the guided import.

## Acceptance criteria

### CLI-008.1 – A second run with a persisted counter
- **Given** a run of
  `framehive import <source> --destination <destination> --template '{seq_stored:4}.{ext}'` that
  copied `DSCF0001.RAF` as `0001.RAF`
- **When** the same command runs again
- **Then** the output has `imported before  DSCF0001.RAF -> 0001.RAF`, the summary
  `0 copied, 1 already imported, 0 duplicates, 0 skipped, 0 failed, 0 left out`, the
  stored counter is still at 1, standard error has no warning, and the exit code is 0

### CLI-008.2 – A file renamed in the library
- **Given** the import of CLI-001.1, after which the target was renamed to
  `2026/2026-10-17/best.RAF`
- **When** the same command runs again
- **Then** the output has `imported before  DSCF0001.RAF -> 2026/2026-10-17/DSCF0001.RAF`,
  and no file is copied

### CLI-008.3 – Backed up before
- **Given** the import of CLI-005.1
- **When** the same command runs again
- **Then** the output has `imported before  DSCF0001.RAF -> …` followed by
  `  backed up before  <backup>`, and the backup summary
  `backup <backup>: 0 backed up, 1 already backed up, 0 failed`

### CLI-008.4 – The history is kept in the state folder
- **Given** `XDG_STATE_HOME` set to an empty folder
- **When** `framehive import` copies a file
- **Then** `$XDG_STATE_HOME/framehive/import-history.sqlite` exists

### CLI-008.5 – A history that cannot be used
- **Given** an `import-history.sqlite` in the state folder that is not a SQLite database
- **When** `framehive import <source> --destination <destination>` runs
- **Then** standard error has
  `error: the import history <path> cannot be used: …`, no file is copied, the file is
  unchanged, and the exit code is 2

### CLI-008.6 – The guided import resumes a pulled card from the history
- **Given** the card pulled during the import of CLI-003.8, with a template that has a
  persisted counter
- **When** the card is inserted again and its import finishes
- **Then** the summary counts the files copied before the card was pulled as
  `already imported`, only the others are copied, and the plan summary says nothing
  about files that are not recognized

### CLI-008.7 – The guided import with a history that cannot be used
- **Given** an `import-history.sqlite` that is not a SQLite database
- **When** the person starts the import of a card
- **Then** the dialog prints `The import history <path> cannot be used: …`, and no file
  is copied

### CLI-008.8 – The guided import with a folder that cannot be resolved
- **Given** an import destination that cannot be resolved to a canonical path
- **When** the person starts the import of a card
- **Then** the dialog prints `Could not use <destination>: …`, and no file is copied

### CLI-008.9 – The template preview with a folder that cannot be resolved
- **Given** an import destination that cannot be resolved to a canonical path
- **When** the person chooses a template
- **Then** the dialog prints `The template cannot be previewed: …` and asks
  `Use this template? [Y/n]: `

### CLI-008.10 – No state folder
- **Given** `XDG_STATE_HOME` unset and `HOME` not an absolute path
- **When** `framehive import <source> --destination <destination> --dry-run` runs
- **Then** standard error has `error: the home folder is unknown; set HOME`, nothing is
  written, and the exit code is 2

### CLI-008.11 – A dry run leaves the state folder as it was
- **Given** a state folder with the history of an earlier import of the source
- **When** `framehive import <source> --destination <destination> --dry-run` runs
- **Then** the dry run shows the file as imported before, and the files of the state
  folder and the content of the history file are unchanged

## Out of scope

- Which files are imported before or backed up before: IMPORT-009.
- Printing or exporting the history as an import log (N6): a later spec.
- A command or an option to import files again that were imported before: not planned
  until it is needed.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-04 | Initial draft; owner decisions: the history in the state folder, resuming is running the import again, no override yet; accepted by the owner |
| 2026-11-04 | Implemented in `framehive-cli`; the template preview shows a file imported before with the line of the file list, CLI-008.6 counts the resumed files in the summary; `implemented` |
| 2026-11-04 | CLI-008.8 and CLI-008.9: the messages for a folder that cannot be resolved |
| 2026-11-04 | CLI-008.10: `framehive import` always needs the state folder; owner decision |
| 2026-11-07 | Examples name the destination with `--destination` (CLI-001, ADR 0036) |
| 2026-11-07 | CLI-008.11: a dry run leaves the state folder as it was |
