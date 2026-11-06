# CLI-014 – Cameras in the guided import

- Status: accepted
- Module: cli (`apps/cli`)
- Origin: Q1, Q2
- Depends on: CLI-002, CLI-003, CLI-013, DEVICES-001, IMPORT-014

## Purpose

The guided import (CLI-002, CLI-003) offers a camera or phone connected over PTP or MTP
as it offers a memory card, so that a camera without a card reader needs no arguments
either (ADR 0023 amendment, ADR 0034 §5).

## Behaviour

**Waiting.** The dialog waits with `Waiting for a memory card or camera…` and polls the
cards and the cameras every second (DEVICES-001). A camera appears as
`[NEW] <model> (camera at <port>)` and goes as `[REMOVED] <model>`. A line typed while
waiting gets `Insert a memory card or connect a camera, or type q to quit.` With several
devices offered, the dialog asks `Which card or camera?`, cards first, as CLI-002 asks
for cards. A camera keeps its identity across a replug by its model; its port changes.

**Opening.** The chosen camera is opened before it is read. When another program holds
it, or it shows no storage, the dialog prints the line of CLI-013 for it, starting with
a capital and ending with a full stop, and asks `Try again? [Y/n]: `. `y` or Enter opens
and reads the camera again; `n` waits for the next device, and the camera is not offered
again until it is disconnected and connected again. Framehive never stops the other
program. Any other failure to open prints `Could not open <model>: <reason>` and waits.

**Reading and the summary.** `Reading <model>…` scans the camera as IMPORT-014 does. A
camera with nothing below `DCIM` prints `<model> has no photos or videos to import.` and
waits. The plan summary, its choices and the import are those of CLI-003 and CLI-006;
the destination cannot lie on a camera, so the summary has no line for a destination
or backup folder on the card. The import history and the import log name the source
`camera:<model>` (CLI-013).

**Report.** After the import, the format readiness (CLI-007) says `camera` and is
followed by the note of IMPORT-014:

```
The camera can be formatted: every file is verified in 1 location.
Only the files below DCIM were checked. Files outside DCIM, such as a phone's other folders, are not backed up.
```

The listing of `l` verifies nothing and prints neither.

**Release.** The dialog then asks `Release <model>? [Y/n]: `. `y` or Enter closes the
camera and prints `<model> can be disconnected now.` On `n` the camera stays open until
it is disconnected or the dialog ends. Either way the dialog waits for the next device,
and the camera is not offered again until it is disconnected and connected again.

**A camera that goes during the import.** A camera that is no longer listed, or whose
storage no longer answers after a file failed, is treated like a pulled card (CLI-003,
IMPORT-004.13): the import stops between files, the dialog prints `[REMOVED] <model>`
and `<model> was disconnected during the import. Connect it again to go on.`, and
releases the camera. When a camera with the same model appears, the dialog reads and
imports it again without asking; files copied before are `already imported`.

## Acceptance criteria

### CLI-014.1 – A camera appears and goes
- **Given** the dialog waiting with a known destination and template
- **When** the camera `Fujifilm X-T5` is connected at `usb:002,007` and later
  disconnected while its summary is shown
- **Then** the output has `Waiting for a memory card or camera…`,
  `[NEW] Fujifilm X-T5 (camera at usb:002,007)`, `Reading Fujifilm X-T5…`, and
  `[REMOVED] Fujifilm X-T5`, and the dialog waits again

### CLI-014.2 – A card and a camera to choose from
- **Given** the card `EOS_DIGITAL` and the camera `Fujifilm X-T5` present at start
- **When** the person answers `2`
- **Then** the dialog lists `  1  EOS_DIGITAL (/run/media/ada/EOS_DIGITAL)` and
  `  2  Fujifilm X-T5 (camera at usb:002,007)` under `Which card or camera?` and reads
  the camera

### CLI-014.3 – Importing a camera
- **Given** a camera with two photos below `DCIM` of one storage
- **When** the person presses Enter at the summary
- **Then** both are copied, the report has
  `The camera can be formatted: every file is verified in 1 location.` followed by the
  note of IMPORT-014, the import history names the source `camera:Fujifilm X-T5`, and
  the summary has no line about a destination on the card

### CLI-014.4 – A camera in use, tried again
- **Given** a camera that another program holds
- **When** it is offered, and the person frees it and answers `y`
- **Then** the output has
  `The camera Fujifilm X-T5 at usb:002,007 is in use by another program; close it (the file manager on Linux, ptpcamerad on macOS) and try again.`
  and `Try again? [Y/n]: `, and the camera is read

### CLI-014.5 – A camera in use, not tried again
- **Given** a camera that another program holds
- **When** the person answers `n` to `Try again? [Y/n]: `
- **Then** the dialog waits and does not offer the camera again until it is disconnected
  and connected again; then it is offered again

### CLI-014.6 – A camera without a storage, tried again
- **Given** a camera that shows no storage
- **When** the person unlocks it and answers `y`
- **Then** the output has
  `The camera Fujifilm X-T5 at usb:002,007 shows no storage; unlock it and choose file transfer (MTP or PTP) on the device, then try again.`
  and `Try again? [Y/n]: `, and the camera is read

### CLI-014.7 – Release on yes
- **Given** a finished camera import
- **When** the person answers `y` to `Release Fujifilm X-T5? [Y/n]: `
- **Then** the camera is closed, the output has
  `Fujifilm X-T5 can be disconnected now.` and `Waiting for a memory card or camera…`,
  and the camera is not offered again while it stays connected

### CLI-014.8 – Kept open on no
- **Given** a finished camera import
- **When** the person answers `n` to the release question
- **Then** the camera stays open until it is disconnected, and is not offered again
  while it stays connected

### CLI-014.9 – Disconnected during the import and resumed
- **Given** a camera with three photos that is disconnected after the first is copied
- **When** a camera with the same model is connected again, at another port
- **Then** the output has `[REMOVED] Fujifilm X-T5` and
  `Fujifilm X-T5 was disconnected during the import. Connect it again to go on.`, the
  camera is read and imported without a question, and the report has
  `2 copied, 1 already imported`

### CLI-014.10 – A camera that stops answering
- **Given** a camera that stays listed but stops answering after the first file
- **When** the import runs
- **Then** it stops as in CLI-014.9 and the camera is released

### CLI-014.11 – A camera with nothing to import
- **Given** a camera without files below `DCIM`
- **When** it is offered
- **Then** the output has `Fujifilm X-T5 has no photos or videos to import.` and the
  dialog waits again

## Out of scope

- Choosing a storage of a camera: every storage is imported (IMPORT-014).
- Stopping the program that holds a camera (ADR 0034 §6).

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-06 | Initial version; owner decisions F1–F4 of 2026-11-06 (E7) |
