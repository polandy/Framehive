# CLI-006 – Backup destinations in the guided import

- Status: implemented
- Module: cli (`apps/cli`)
- Origin: I2, I5
- Depends on: CLI-003, CLI-005, IMPORT-007

## Purpose

The guided import (CLI-003) writes each file to the backup destinations the person
chose once (IMPORT-007), so every card is backed up without being asked again.

## Behaviour

**Plan summary.** Below the `to` line, the summary has one line per backup destination,
in their order, and the prompt gets `[b] backup`:

```
EOS_DIGITAL: 1024 files, 12.8 GB, captured 2026-10-17 to 2026-10-19
  to        /home/ada/Pictures
  backup    /run/media/ada/Backup/Pictures
  named     {date:%Y}/{date:%Y-%m-%d}/{name}.{ext}
[Enter] import  [l] list  [t] template  [d] destination  [b] backup  [q] quit: 
```

Without backup destinations, the summary has the line `  backup    none`.

**Choosing backup destinations.** `b` prints the backup destinations numbered from 1 and
asks `Backup folder to add, or the number of one to remove [Enter keeps them]: `. A
folder is added at the end; a number removes that backup destination; Enter keeps them.
The dialog then plans again and shows the summary. The first run does not ask for backup
destinations; it starts with none.

**Invalid backup destinations.** A backup destination that lies on the card, or that is
invalid by IMPORT-007.11, makes the summary say
`The backup folder <folder> cannot be used: <reason>; type b to change it.`, and Enter
does not start. A backup destination that cannot be created when the import starts
makes the dialog say `Could not create <folder>: <reason>`, and nothing is copied.

**Import and report.** The import runs as in CLI-005, and the progress line counts the
bytes read from the card, not the bytes written to each destination. After the import,
the report of CLI-003 also has the line of each file with a backup failure, followed by
its `  backup failed  …` line (CLI-005), and after the summary line the backup summary
lines of CLI-005.

**Remembered state.** The backup destinations are remembered in the state file with the
key `backup`, in their order (ADR 0030), whenever they change.

## Acceptance criteria

### CLI-006.1 – Adding a backup destination
- **Given** the plan summary with no backup destination
- **When** the person types `b` and answers `/run/media/ada/Backup/Pictures`
- **Then** the summary shows `  backup    /run/media/ada/Backup/Pictures`, and the
  state file has the line `backup=/run/media/ada/Backup/Pictures`

### CLI-006.2 – Removing a backup destination
- **Given** the backup destinations `/mnt/a` and `/mnt/b`
- **When** the person types `b` and answers `1`
- **Then** the summary shows only `  backup    /mnt/b`, and it is remembered

### CLI-006.3 – A backup destination on the card
- **Given** the backup destination `/run/media/ada/EOS_DIGITAL/backup` and that card
- **When** the plan summary is shown and the person presses Enter
- **Then** the summary says the backup folder cannot be used, and nothing is copied

### CLI-006.4 – Import with a backup destination
- **Given** a card with two files, a backup destination, and the plan summary
- **When** the person presses Enter
- **Then** both files are in the destination and in the backup destination, and the
  report ends with `backup <folder>: 2 backed up, 0 already backed up, 0 failed`

### CLI-006.5 – A failed backup is reported
- **Given** a backup destination whose target for the first of two files holds other
  content
- **When** the import runs
- **Then** the report has the `copied  …` line of that file, followed by
  `  backup failed  <folder>: name conflict`, and the backup summary counts 1 backed up
  and 1 failed

### CLI-006.6 – Remembered backup destinations
- **Given** a run in which the backup destination `/mnt/backup` was added
- **When** the guided import starts again with a card
- **Then** the summary shows `  backup    /mnt/backup` without asking

### CLI-006.7 – A state file of version 1
- **Given** a state file with `format_version=1` and a destination
- **When** the guided import starts
- **Then** it uses the destination without a warning, has no backup destination, and
  the next write stores `format_version=2`

### CLI-006.8 – A backup destination that cannot be created
- **Given** the card of CLI-006.4, and the backup destination `<file>/backup`, where
  `<file>` is a file
- **When** the person presses Enter
- **Then** the dialog says `Could not create <file>/backup: …`, and no file is copied

## Out of scope

- The "card can be formatted" status (N5): CLI-007.
- Backup drives recognized by a marker folder (I3): a later spec.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-04 | Initial draft for backup destinations (IMPORT-007, ADR 0030); accepted by the owner |
| 2026-11-04 | Implemented in `apps/cli/src/guided` |
| 2026-11-07 | Add CLI-006.8: the import job creates the backup destinations when it starts (IMPORT-012) |
