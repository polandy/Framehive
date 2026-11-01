# CLI-003 – Guided import run

- Status: implemented
- Module: cli (`apps/cli`)
- Origin: Q1, A3, R9, N9, X1
- Depends on: CLI-002, CLI-001, DEVICES-001, IMPORT-003, IMPORT-004, IMPORT-006

## Purpose

The guided import (CLI-002) shows what it will do with a card, imports the card on Enter
with a progress line, reports and ejects it, and remembers the answers, so the next card
needs only Enter (ADR 0024, ADR 0025).

## Behaviour

### Plan summary

```
EOS_DIGITAL: 1024 files, 12.8 GB, captured 2026-10-17 to 2026-10-19
  to        /home/ada/Pictures
  named     {date:%Y}/{date:%Y-%m-%d}_{job}/{name}.{ext}
  job code  Smith wedding
[Enter] import  [l] list  [j] job code  [t] template  [d] destination  [q] quit: 
```

- The capture dates are the first and the last local date of the planned files; one day
  is written `captured 2026-10-17`. The `job code` line and `[j]` appear only when the
  template has `{job}`.
- Sizes are decimal: `12.8 GB` from one gigabyte (10⁹ bytes) on, otherwise `<n.n> MB`.
- Enter starts the import.
- `l` prints the lines of `framehive import --dry-run` (CLI-001) for the planned files,
  and the summary again.
- `j` asks for the job code, `t` runs the template step, and `d` asks for the
  destination, with the current one as default; each then plans again and shows the
  summary.
- When the destination lies inside the card, the summary says
  `The destination is on the card; type d to choose another.` and Enter does not start.

### Import and progress

The job code is remembered when the import starts. The import runs as in CLI-001, with
compatibility mode on and the conflict policy `add_suffix`. One line shows the progress
and is redrawn in place after each file:

```
  312/1024 files  4.1/12.8 GB  85 MB/s  1:42 left
```

The rate is the bytes of the finished files divided by the time since the import
started, by the clock, in whole MB/s; the time left is the remaining bytes divided by the rate,
written `m:ss`, or `h:mm:ss` from one hour on. Until the rate is known, the line reads
`-- MB/s  --:-- left`.

### Report and eject

After the import, the line `failed  …` or `skipped  …` of CLI-001 is printed for each
file that failed or was skipped, then the summary line of CLI-001. The dialog asks
`Eject <name>? [Y/n]: `. On `y`, the card is ejected and the dialog prints
`<name> can be removed now.`; when the eject fails, it prints
`Could not eject <name>: <reason>`. Then it waits for the next card.

### A card pulled during the import

When the card is pulled during the import, the import stops before the next file
(IMPORT-004), and the dialog prints `[REMOVED] <name>` and
`<name> was removed during the import. Insert it again to go on.` When a card with the
same name appears, the dialog plans the whole card again and imports it without asking;
files copied before are then `already imported` (IMPORT-003). Other cards are listed but
not offered meanwhile.

### Remembered state

The destination, the template and the job codes are remembered as ADR 0025 describes. A
state file that cannot be read or understood prints a `warning:` line to standard error
and the dialog starts as on the first run; a failed write prints a `warning:` line and
the dialog goes on.

## Acceptance criteria

### CLI-003.1 – Plan summary
- **Given** a card `EOS_DIGITAL` with two files of 1 500 000 000 bytes each, captured
  on 2026-10-17 and 2026-10-19, and the template of preset 2 with the job code
  `Smith wedding`
- **When** the plan summary is shown
- **Then** it reads
  `EOS_DIGITAL: 2 files, 3.0 GB, captured 2026-10-17 to 2026-10-19`, with the
  destination, template and job code lines, and the prompt with `[j]`

### CLI-003.2 – Listing the files
- **Given** the plan summary for a card whose first file is already imported
- **When** the person types `l`
- **Then** the dialog prints `already imported  …` for it and `would copy  …` for the
  others, as CLI-001 does with `--dry-run`, copies nothing, and shows the summary again

### CLI-003.3 – Changing the destination
- **Given** the plan summary
- **When** the person types `d` and answers `/home/ada/Archive`
- **Then** the summary shows `/home/ada/Archive`, and it is remembered

### CLI-003.4 – Destination on the card
- **Given** the destination `/run/media/ada/EOS_DIGITAL/import` and that card
- **When** the plan summary is shown and the person presses Enter
- **Then** the summary says the destination is on the card, and nothing is copied

### CLI-003.5 – Import, progress and report
- **Given** a card with three files of 10 MB each, a clock that advances one second
  per file, and the plan summary
- **When** the person presses Enter
- **Then** the files are copied and verified as in CLI-001, the progress line is
  redrawn after each file and its last state reads
  `  3/3 files  30.0/30.0 MB  10 MB/s  0:00 left`, and the summary line of CLI-001
  follows

### CLI-003.6 – Eject
- **Given** a finished import of `EOS_DIGITAL`
- **When** the person presses Enter at `Eject EOS_DIGITAL? [Y/n]: `
- **Then** the card is ejected, the dialog prints `EOS_DIGITAL can be removed now.` and
  `Waiting for a memory card…`

### CLI-003.7 – Eject fails
- **Given** a finished import and a device source whose eject fails with `target is busy`
- **When** the person answers `y`
- **Then** the dialog prints `Could not eject EOS_DIGITAL: target is busy` and waits for
  the next card

### CLI-003.8 – Card pulled during the import
- **Given** a card with three files that is pulled after the first file is copied
- **When** the import runs, and the card appears again
- **Then** the import stops before the third file, the dialog asks to insert the card
  again, and after it appears the first file is reported `already imported` and the
  others are copied

### CLI-003.9 – Remembered answers
- **Given** a run in which the destination, preset 2 and the job code `Harbour` were
  chosen and an import started
- **When** the guided import starts again with a card
- **Then** it asks neither for the destination nor for the template, and offers
  `Harbour` as the default job code

### CLI-003.10 – A broken state file
- **Given** a state file with `format_version=7`
- **When** the guided import starts
- **Then** it prints a `warning:` line to standard error and asks for the destination as
  on the first run

### CLI-003.11 – State file format
- **Given** the destination `/home/ada/Pictures`, the template of preset 2 and the job
  codes `Smith wedding`, `Harbour`
- **When** the state is written
- **Then** the file at `$XDG_STATE_HOME/framehive/guided-import.txt` has the lines of
  ADR 0025 in that order, and reading it gives the same state

### CLI-003.12 – Where the state file is
- **Given** `XDG_STATE_HOME` not set, and in another run set to `relative/path`
- **When** the state file is located
- **Then** it is `<home>/.local/state/framehive/guided-import.txt` in both runs

## Out of scope

- The arguments, output and exit codes of `framehive import`: CLI-001.
- Import profiles and the YAML settings (N8): later specs; they will supersede the
  remembered template and destination.
- Backup destinations (I2), "card can be formatted" (N5) and resuming an interrupted
  import across program runs (N4): later specs.
- Ctrl+C during a copy: the process ends, and the next import replaces the leftover
  `.part` file (IMPORT-004.10).

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-01 | Initial draft as part of CLI-002 (guided import plan, PR 4) |
| 2026-11-01 | Accepted by the owner as part of CLI-002 |
| 2026-11-01 | Implemented in `apps/cli/src/guided`; `implemented` |
| 2026-11-01 | Split from CLI-002 to stay under the document size limit; criteria renumbered, content unchanged |
