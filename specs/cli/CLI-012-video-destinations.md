# CLI-012 – Video destinations in `framehive import`

- Status: implemented
- Module: cli (`apps/cli`)
- Origin: R7, I4, N18
- Depends on: CLI-001, CLI-005, CLI-010, IMPORT-013

## Purpose

`framehive import` sends the videos of a card to a folder and backup folders of their
own (IMPORT-013), given on the command line or in an import profile, so a script imports
photos and videos in one run.

## Behaviour

```
framehive import <source> <destination> [--video-destination <folder>]
                 [--video-backup <folder>]... [other arguments]
```

`--video-destination <folder>` sets the video destination. `--video-backup <folder>`
adds a video backup destination; it can be given more than once and keeps the order of
the command line. Without either option, the command behaves as CLI-005 and CLI-010
describe, and videos go where photos go.

**Settings.** The rule of CLI-010, "Settings", holds for two more rows: an option on the
command line replaces the profile's value, also for the list.

| Setting | Option | Profile key |
|---|---|---|
| Video destination | `--video-destination` | `video_destination` |
| Video backup destinations | every `--video-backup` | `backup.video_destinations` |

**Report.** When a video destination is set, the line of each video (CLI-001) ends with
`  (video destination)`, after any other note of the line, so its target path is read
below the video destination. Every line of such a video has the mark, also a duplicate or
a failure. A video destination that is the import destination marks no line. The
backup lines of a file (CLI-005) name the folders of that file's backup destinations.
After the summary, the lines of CLI-005 list each backup destination and then each
video backup destination that is not a backup destination too; a folder that is both
has one line that counts photos and videos.

With `--dry-run`, the video destination and video backup destinations are checked as
before a run (IMPORT-013, "Before the run"), and nothing is written to them.

**Exit codes.** An invalid video destination or video backup destination (IMPORT-013.8),
or one that cannot be created, stops the command before any file is copied, with exit
code 2. A failure at a video backup destination makes the exit code 1, as in CLI-005.
Too little free space at a video destination is reported as in CLI-011.

The guided import (CLI-002) has no video destinations (IMPORT-013, "Out of scope").

## Acceptance criteria

### CLI-012.1 – Import with a video destination
- **Given** a source with `DSCF0001.RAF` and `clip.MOV` taken on 2026-10-17, and empty
  folders `<destination>` and `<videos>`
- **When** the user runs `framehive import <source> <destination> --video-destination
  <videos>`
- **Then** `<destination>` has `2026/2026-10-17/DSCF0001.RAF`, `<videos>` has
  `2026/2026-10-17/clip.MOV`, the line of the video starts with `copied  clip.MOV ->
  2026/2026-10-17/clip.MOV` and ends with `  (video destination)`, and the exit code is 0

### CLI-012.2 – Video backup destinations
- **Given** the source and folders of CLI-012.1, and empty folders `<backup>` and
  `<video-backup>`
- **When** the user adds `--backup <backup> --video-backup <video-backup>`
- **Then** `<backup>` has only the photo, `<video-backup>` has only the video, the line
  of `clip.MOV` is followed by `  backed up  <video-backup>`, and the summary is followed
  by `backup <backup>: 1 backed up, 0 already backed up, 0 failed` and
  `backup <video-backup>: 1 backed up, 0 already backed up, 0 failed`

### CLI-012.3 – One folder for both backups
- **Given** the source and folders of CLI-012.1, and an empty folder `<backup>`
- **When** the user adds `--backup <backup> --video-backup <backup>`
- **Then** `<backup>` has both files, and the summary is followed by exactly one line
  `backup <backup>: 2 backed up, 0 already backed up, 0 failed`

### CLI-012.4 – The profile supplies the video settings
- **Given** a profile `holiday` with `video_destination` and
  `backup.video_destinations` set to empty folders
- **When** the user runs `framehive import --profile holiday <source> <destination>`
- **Then** the video goes to the profile's video destination and its video backup
  destination; and with `--video-destination <other>` on the command line, it goes to
  `<other>` instead

### CLI-012.5 – An invalid video destination stops the command
- **Given** a video destination inside the source
- **When** the user runs the command
- **Then** standard error names the folder, nothing is written, and the exit code is 2

## Out of scope

- The rules of the destinations themselves: IMPORT-013.
- The keys of an import profile and their checks: IMPORT-011.
- How the import log shows the video destination: CLI-009.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-06 | Initial draft; owner decisions: options `--video-destination` and `--video-backup`, profile keys `video_destination` and `backup.video_destinations`; accepted by the owner |
| 2026-11-06 | The mark ends the line after any other note; CLI-012.1 matches the start and end of the line |
