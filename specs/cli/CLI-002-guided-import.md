# CLI-002 – Guided import dialog

- Status: implemented
- Module: cli (`apps/cli`)
- Origin: Q1, A3, R9, N9, X1
- Depends on: CLI-001, DEVICES-001, IMPORT-001, IMPORT-005, IMPORT-006

## Purpose

A person who comes home with a full memory card should not need to know mount points
or arguments. `framehive` without arguments waits for a card, shows what it will do,
imports the card on Enter and ejects it. It remembers the answers, so the next card
needs only Enter (ADR 0024, ADR 0025).

The plan summary, the import, the eject and the remembered state are specified in
CLI-003.

## Behaviour

### Start

`framehive` without a subcommand starts the guided import. When standard input or
standard output is not a terminal, it writes
`error: the guided import needs a terminal; use framehive import <source> <destination> in scripts`
to standard error and exits with code 2.

It reads the remembered state (ADR 0025) and prints
`Framehive guided import. Type q and Enter to quit.`

### Prompts

The dialog prints lines to standard output and reads whole lines. Each prompt shows its
default in brackets, and an empty line takes it. Surrounding whitespace of a typed line
is ignored. At every prompt, `q` quits, and so does the end of input (Ctrl+D); Framehive
then exits with code 0. An answer the prompt does not accept prints one line that says
why, and the prompt is repeated.

While the dialog waits for a line, it polls the devices once a second (DEVICES-001), so
that inserted and pulled cards are noticed at once.

### Destination

When no destination is remembered, the dialog asks
`Where should the photos go? [<home>/Pictures]: `. A leading `~/` stands for the home
folder; a relative path is taken relative to the current folder. The answer is
remembered. A missing destination is created with its parents when the first import
starts, as in CLI-001.

### Waiting for a card

The dialog prints `Waiting for a memory card…` and lists each card as it appears:
`[NEW] <name> (<mount point>)`. When a card is pulled, it prints
`[REMOVED] <name>`. Cards that are present when the dialog starts count as appearing.

A card is offered for import when it appears. A card that was imported, or that had
nothing to import, is not offered again until it is pulled and inserted again.

When one card is offered, it is chosen. When several are offered at the same poll, the
dialog asks:

```
Which card?
  1  EOS_DIGITAL (/run/media/ada/EOS_DIGITAL)
  2  Untitled (/run/media/ada/Untitled)
Choose 1–2 [1]: 
```

The dialog then prints `Reading <name>…`, scans the card (IMPORT-005) and plans every
file (IMPORT-001). A card without files to import prints
`<name> has no photos or videos to import.` and the dialog waits again.

### Template step

When no template is remembered, the template step runs before the first plan; later it
runs when the person types `t` at the plan summary.

```
How should the photos be named?
  1  By day:              {date:%Y}/{date:%Y-%m-%d}/{name}.{ext}
  2  By day and job code: {date:%Y}/{date:%Y-%m-%d}_{job}/{name}.{ext}
  3  By month, renamed:   {date:%Y}/{date:%Y-%m}/{date:%Y%m%d}_{number:4}.{ext}
Choose 1–3 or type a template [1]: 
```

The default is the remembered template's preset, or 1. Any answer other than a number
1–3 is a template. A template that does not parse (IMPORT-001) is printed again with a
caret under the column of the error, and the error after it:

```
  {date:%Y}/{camera}/{name}.{ext}
            ^ <the error of IMPORT-001>
```

and the step starts again. A valid template is previewed with the first five files of
the card, planned with the current job code, or with the job code `job-code` when there
is none yet:

```
Preview with the first 5 files of EOS_DIGITAL:
  DCIM/100_FUJI/DSCF0001.RAF -> 2026/2026-10-17/DSCF0001.RAF
  DCIM/100_FUJI/DSCF0002.RAF -> cannot be named: <reason>
Use this template? [Y/n]: 
```

`y` takes the template and remembers it; `n` starts the step again.

### Job code

When the template has `{job}`, the dialog asks for a job code before the plan summary,
for each card. The remembered job codes (IMPORT-006) are offered, the most recent first:

```
Job code:
  1  Smith wedding
  2  Harbour
Choose 1–2 or type a job code [1]: 
```

A number of the list chooses that job code; any other answer is a new job code, checked
by IMPORT-006. Without remembered job codes, the prompt is `Job code: ` and has no
default; an empty answer prints `A job code is needed, because the template has {job}.`
When the template has no `{job}`, the dialog never asks for a job code and the import
has none.

### Exit codes

| Code | Meaning |
|---|---|
| 0 | The person quit, by `q` or the end of input |
| 2 | The guided import cannot start: standard input or standard output is not a terminal, or the home folder or the current folder is unknown |

## Acceptance criteria

### CLI-002.1 – Not a terminal
- **Given** standard input that is not a terminal
- **When** `framehive` runs without arguments
- **Then** it prints the error that points to `framehive import`, reads no state and
  exits with code 2

### CLI-002.2 – First run asks for the destination
- **Given** no state file and the home folder `/home/ada`
- **When** the guided import starts and the person presses Enter
- **Then** the prompt `Where should the photos go? [/home/ada/Pictures]: ` was shown,
  and `/home/ada/Pictures` is remembered

### CLI-002.3 – `~/` and relative destinations
- **Given** the home folder `/home/ada` and the current folder `/tmp/work`
- **When** the person answers `~/Fotos`, and in another run `cards`
- **Then** the destination is `/home/ada/Fotos`, and `/tmp/work/cards`

### CLI-002.4 – Waiting lists cards as they come and go
- **Given** a remembered destination and template and no card
- **When** the card `EOS_DIGITAL` appears at the third poll and is pulled at the fifth
  before any line is typed
- **Then** the output shows `Waiting for a memory card…`, then
  `[NEW] EOS_DIGITAL (/run/media/ada/EOS_DIGITAL)` and `[REMOVED] EOS_DIGITAL`

### CLI-002.5 – One card is chosen, several are asked for
- **Given** one card at the start, and in another run two cards at the start
- **When** the guided import starts
- **Then** the single card is read at once, and for two cards `Which card?` lists both
  numbered by mount point, and Enter chooses the first

### CLI-002.6 – A card with nothing to import
- **Given** a card whose `DCIM` folder has no files to import
- **When** it is chosen
- **Then** the dialog prints `EOS_DIGITAL has no photos or videos to import.`, waits
  again, and does not offer the card until it is pulled and inserted again

### CLI-002.7 – Template presets
- **Given** no remembered template and a card with `DSCF0001.RAF` captured at
  2026-10-17 14:05:09
- **When** the person answers `3`, then `y`
- **Then** the preview line reads `DCIM/100_FUJI/DSCF0001.RAF -> 2026/2026-10/20261017_0001.RAF`,
  and the template of preset 3 is remembered

### CLI-002.8 – Own template with an error
- **Given** the template step
- **When** the person types `{date:%Y}/{camera}/{name}.{ext}`
- **Then** the template is printed with a caret under column 11 and the error of
  IMPORT-001, and the presets are shown again

### CLI-002.9 – Rejecting the preview
- **Given** a valid template in the preview
- **When** the person answers `n`
- **Then** the template is not remembered and the presets are shown again

### CLI-002.10 – Preview with five files
- **Given** a card with seven files and the template step
- **When** a valid template is chosen
- **Then** the preview lists the first five files in scan order, and a file that cannot
  be named shows `cannot be named:` with the reason

### CLI-002.11 – No job code prompt without `{job}`
- **Given** the template `{date:%Y}/{date:%Y-%m-%d}/{name}.{ext}`
- **When** a card is chosen
- **Then** the dialog shows the plan summary without asking for a job code, and the
  summary has no `job code` line and no `[j]`

### CLI-002.12 – Choosing a remembered job code
- **Given** a template with `{job}` and the remembered job codes `Smith wedding`,
  `Harbour`
- **When** the person answers `2` to the job code prompt and starts the import
- **Then** the files are named with `Harbour`, and the remembered list is `Harbour`,
  `Smith wedding`

### CLI-002.13 – A new job code is checked
- **Given** a template with `{job}` and no remembered job codes
- **When** the person answers an empty line, then `Smith/Jones`, then `Smith wedding`
- **Then** the dialog says a job code is needed, then prints the IMPORT-006 error that
  names `/`, then takes `Smith wedding`

### CLI-002.14 – Quit
- **Given** any prompt
- **When** the person types `q`, or the input ends
- **Then** Framehive exits with code 0 without copying anything further

## Out of scope

- The arguments, output and exit codes of `framehive import`: CLI-001.
- Import profiles and the YAML settings (N8): later specs; they will supersede the
  remembered template and destination.
- Cameras over PTP or MTP (Q2): further device sources.
- A graphical template editor with live preview (N9, R10): the GUI.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-01 | Initial draft (guided import plan, PR 4) |
| 2026-11-01 | Accepted by the owner |
| 2026-11-01 | Implemented in `apps/cli/src/guided`; `implemented` |
| 2026-11-01 | Split from CLI-002 to stay under the document size limit; criteria renumbered, content unchanged |
| 2026-11-01 | Exit code 2 also covers an unknown home folder or current folder, as implemented |
