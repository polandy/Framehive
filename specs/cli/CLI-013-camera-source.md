# CLI-013 – Cameras in `framehive import`

- Status: implemented
- Module: cli (`apps/cli`)
- Origin: Q2, N18
- Depends on: CLI-001, CLI-007, CLI-009, CLI-010, IMPORT-014

## Purpose

`framehive import` imports from a camera or phone connected over PTP or MTP as it
imports a card, and `framehive cameras` shows which cameras are connected
(ADR 0034 §5). The guided import offers cameras too: CLI-014.

## Behaviour

```
framehive cameras
framehive import camera:[<model>|<port>] [--destination <folder>] [other arguments of CLI-001]
```

**Naming a camera.** A source that starts with `camera:` names a camera; any other
source is a folder as before. A folder whose name starts with `camera:` is given as
`./camera:…`. After `camera:`:

| Text | Camera |
|---|---|
| empty | The only connected camera |
| contains a `:` | The camera at that port, for example `usb:001,005`; the port is opened even if it is not listed |
| anything else | The connected camera whose model is the text, ignoring ASCII case |

**`framehive cameras`.** Standard output gets one line per connected camera, sorted by
port in byte order:

```
<model>  <port>
```

Without a camera, standard error gets `no camera is connected`, and the exit code is 0.
A failure to list the cameras goes to standard error with exit code 2.

**Errors that stop the import** go to standard error, nothing is created, and the exit
code is 2:

| Line | When |
|---|---|
| `no camera is connected` | `camera:` and no camera |
| `several cameras are connected; name one: camera:<port> (<model>), …` | `camera:` and more than one camera, listed by port |
| `no camera <model> is connected` | No camera has that model |
| `several cameras <model> are connected; name one: camera:<port>, …` | More than one camera has that model |
| `cannot open the camera at <port>: <reason>` | The camera cannot be opened |
| `the camera <model> at <port> is in use by another program; close it (the file manager on Linux, ptpcamerad on macOS) and try again` | Another program holds the camera |
| `the camera at <port> is in use by another program; close it (the file manager on Linux, ptpcamerad on macOS) and try again` | Another program holds a camera given by a port that is not listed |
| `the camera <model> at <port> shows no storage; unlock it and choose file transfer (MTP or PTP) on the device, then try again` | The camera shows no storage (IMPORT-014.11), for example a locked phone or one set to charge only |
| `the camera at <port> shows no storage; unlock it and choose file transfer (MTP or PTP) on the device, then try again` | A camera given by a port that is not listed shows no storage |

**The run.** The import runs as in CLI-001 with the files of IMPORT-014; paths in the
report are relative to the camera's root. A camera that stops answering during the run
fails the remaining files, and the exit code is 1. The format readiness (CLI-007) says
`source`, and after its lines a camera run that is not a dry run prints the note of
IMPORT-014:

```
Only the files below DCIM were checked. Files outside DCIM, such as a phone's other folders, are not backed up.
```

The camera is released when the command ends. No destination is checked against a
camera source, because none can lie on it.

**History and log.** The import log (CLI-009) shows the source of a camera run as
`camera:<model>`, with the model the opened camera reports. A camera opened by a port
that `framehive cameras` does not list is shown as `camera:<port>`.

**Profiles.** A profile (CLI-010) supplies the destination and settings for a camera
source as for a folder.

## Acceptance criteria

### CLI-013.1 – `framehive cameras` lists the connected cameras
- **Given** the cameras `Fujifilm X-T5` at `usb:002,007` and `Fujifilm X-T2` at
  `usb:001,004`
- **When** the user runs `framehive cameras`
- **Then** standard output is `Fujifilm X-T2  usb:001,004` and then
  `Fujifilm X-T5  usb:002,007`, and the exit code is 0

### CLI-013.2 – Without a camera, the list is empty
- **Given** no connected camera
- **When** the user runs `framehive cameras`
- **Then** standard output is empty, standard error is `no camera is connected`, and the
  exit code is 0

### CLI-013.3 – A camera given by its port is imported
- **Given** a camera at the port `disk:<folder>` (the `directory` driver) whose storage
  holds `DCIM/100_FUJI/DSCF0001.RAF` of 2026-10-17
- **When** the user runs `framehive import camera:disk:<folder> --destination <destination>`
- **Then** standard output has a line
  `copied  <storage>/DCIM/100_FUJI/DSCF0001.RAF -> 2026/2026-10-17/DSCF0001.RAF`, the file
  is at that target, and the exit code is 0

### CLI-013.4 – `camera:` takes the only camera
- **Given** one connected camera `Fujifilm X-T5` with `DCIM/100_FUJI/DSCF0001.RAF`
- **When** the user runs `framehive import camera: --destination <destination>`
- **Then** `DSCF0001.RAF` is copied, and the exit code is 0

### CLI-013.5 – `camera:` without a camera stops
- **Given** no connected camera
- **When** the user runs `framehive import camera: --destination <destination>`
- **Then** standard error has `no camera is connected`, the destination is not created,
  and the exit code is 2

### CLI-013.6 – `camera:` with several cameras stops
- **Given** the cameras of CLI-013.1
- **When** the user runs `framehive import camera: --destination <destination>`
- **Then** standard error has
  `several cameras are connected; name one: camera:usb:001,004 (Fujifilm X-T2), camera:usb:002,007 (Fujifilm X-T5)`,
  and the exit code is 2

### CLI-013.7 – A camera is chosen by its model, ignoring case
- **Given** the cameras of CLI-013.1
- **When** the user runs `framehive import camera:fujifilm\ x-t5 --destination <destination>`
- **Then** the files of the camera at `usb:002,007` are imported

### CLI-013.8 – An unknown or ambiguous model stops
- **Given** the cameras of CLI-013.1, and in a second case two cameras `Fujifilm X-T5`
  at `usb:001,004` and `usb:002,007`
- **When** the user runs `framehive import camera:Canon --destination <destination>`, and in the second
  case `framehive import camera:Fujifilm\ X-T5 --destination <destination>`
- **Then** standard error has `no camera Canon is connected`, and in the second case
  `several cameras Fujifilm X-T5 are connected; name one: camera:usb:001,004, camera:usb:002,007`;
  each exit code is 2

### CLI-013.9 – A camera in use stops with a remedy
- **Given** a camera `Fujifilm X-T5` at `usb:002,007` that another program holds
- **When** the user runs `framehive import camera: --destination <destination>`
- **Then** standard error has
  `the camera Fujifilm X-T5 at usb:002,007 is in use by another program; close it (the file manager on Linux, ptpcamerad on macOS) and try again`,
  nothing is created, and the exit code is 2

### CLI-013.10 – A port that cannot be opened stops
- **Given** no camera at `usb:009,009`
- **When** the user runs `framehive import camera:usb:009,009 --destination <destination>`
- **Then** standard error has a line that starts with
  `cannot open the camera at usb:009,009: `, and the exit code is 2

### CLI-013.11 – A camera that goes away ends with exit code 1
- **Given** a camera with two files that stops answering after the first is copied
- **When** the user runs `framehive import camera: --destination <destination>`
- **Then** the second file has a `failed` line, the output has
  `Do not format the source: not every file is verified in 1 location.`, and the exit
  code is 1

### CLI-013.12 – The import log names the camera
- **Given** the import of CLI-013.4
- **When** the user runs `framehive import-log 1`
- **Then** the line `source               camera:Fujifilm X-T5` is printed

### CLI-013.13 – A folder named like a camera is imported as a folder
- **Given** a folder `camera:x` in the working folder with `DSCF0001.RAF`
- **When** the user runs `framehive import ./camera:x --destination <destination>`
- **Then** `DSCF0001.RAF` is copied, and no camera is opened

### CLI-013.14 – A camera run notes that only DCIM was checked
- **Given** the camera of CLI-013.4
- **When** the user runs `framehive import camera: --destination <destination>`, and in a second case
  with `--dry-run`
- **Then** the line after the format readiness is
  `Only the files below DCIM were checked. Files outside DCIM, such as a phone's other folders, are not backed up.`,
  and the dry run does not print it

### CLI-013.15 – A camera without a storage stops with a remedy
- **Given** a connected camera `Fujifilm X-T5` at `usb:002,007` that shows no storage
- **When** the user runs `framehive import camera: --destination <destination>`
- **Then** standard error has
  `the camera Fujifilm X-T5 at usb:002,007 shows no storage; unlock it and choose file transfer (MTP or PTP) on the device, then try again`,
  nothing is created, and the exit code is 2

## Out of scope

- `--format json` for `framehive cameras`: not needed yet.
- Stopping the program that holds a camera.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-06 | Initial version; owner decisions E1–E7 of 2026-11-06 (ADR 0034) |
| 2026-11-06 | CLI-013.14: the note of IMPORT-014.10 after the format readiness (owner decision E8); `camera:<port>` names an unlisted camera; implemented |
| 2026-11-06 | CLI-013.15: a camera without a storage stops with exit code 2 and a remedy (owner decision E10) |
| 2026-11-06 | Cross-reference to CLI-014 (cameras in the guided import) |
| 2026-11-07 | Examples name the destination with `--destination` (CLI-001, ADR 0036) |
