# IMPORT-014 – Cameras and phones as a source

- Status: implemented
- Module: import
- Origin: Q2, N26
- Depends on: IMPORT-004, IMPORT-005, IMPORT-008, IMPORT-009

## Purpose

A camera or phone connected by USB over PTP or MTP is not a mounted card. The import
must read its photos and videos all the same, with the same plan, verified copy, history
and format readiness as a card, and must never change anything on the camera (ADR 0034).

## Behaviour

A camera is opened through the port `files::Cameras` and read through
`files::ReadOnlyFileSystem`. Its file system has one top-level folder per storage, named
as the camera reports it, for example `store_00010001`.

**What is scanned.** On a camera, only the folder `DCIM` at the top of each storage is
scanned, with its subfolders, by the rules of IMPORT-005 (roles, hidden files, symbolic
links, unreadable folders, order). Everything outside `DCIM` is not looked at and not
listed as left out. A storage without `DCIM` gives no files and is not an error, and
the format readiness is that of an empty card (IMPORT-008.13). A `DCIM` that cannot be
looked at is left out as "cannot be read", so the run is not ready to be formatted.

**A camera without a storage.** A camera whose file system has no storage folder at all
cannot be scanned: the scan stops with an error that says the camera shows no storage.
A phone does this when it is locked or set to charge only, so an empty result would
wrongly suggest that there is nothing to import. The path of a found file is relative to the camera's root
and starts with its storage folder, for example
`store_00010001/DCIM/100_FUJI/DSCF0001.RAF`.

**What is the same as for a folder.** Planning, naming, conflicts, the verified copy,
backup and video destinations, the free-space check and the import history work as for a
folder source. The source path in the history is the path above.

**Read only.** The import never creates, changes or removes anything on a camera.

**A camera that goes away.** When the camera stops answering during a run, for example
because it was unplugged, every file that is not yet copied fails with the reason the
camera gives, the run ends, and the run is not ready to be formatted (IMPORT-008).

**Format readiness.** The format readiness of a camera run covers the files below `DCIM`
of each storage, because those are all the import looks at. A phone keeps files outside
`DCIM` that the import neither copies nor backs up, so the format readiness of a camera
run says so in a note of its own, after the verdict:

> Only the files below DCIM were checked. Files outside DCIM, such as a phone's other
> folders, are not backed up.

## Acceptance criteria

### IMPORT-014.1 – Files below DCIM are found with their storage folder
- **Given** a camera with the storage `store_00010001` holding
  `DCIM/100_FUJI/DSCF0001.RAF`
- **When** the camera is scanned
- **Then** the file `store_00010001/DCIM/100_FUJI/DSCF0001.RAF` is found

### IMPORT-014.2 – Files outside DCIM are not looked at
- **Given** a camera storage with `DCIM/100_FUJI/DSCF0001.RAF`, `Music/song.mp4` and
  `Download/photo.jpg`
- **When** the camera is scanned
- **Then** only `DSCF0001.RAF` is found, and no file or folder outside `DCIM` is listed
  as left out

### IMPORT-014.3 – Every storage is scanned
- **Given** a camera with the storages `store_00010001` and `store_00020001`, each with a
  file below `DCIM`
- **When** the camera is scanned
- **Then** both files are found, in byte order of their paths

### IMPORT-014.4 – A storage without DCIM is empty, not an error
- **Given** a camera with one storage without `DCIM`
- **When** it is scanned
- **Then** the scan succeeds and finds no files

### IMPORT-014.5 – The scan rules of a folder apply below DCIM
- **Given** a camera storage with `DCIM/100_FUJI/DSCF0001.RAF`,
  `DCIM/100_FUJI/._DSCF0001.RAF` and `DCIM/100_FUJI/notes.txt`
- **When** the camera is scanned
- **Then** only `DSCF0001.RAF` is found, `._DSCF0001.RAF` is left out as "hidden" and
  `notes.txt` as "unsupported type"

### IMPORT-014.6 – A camera import is copied, verified and recorded
- **Given** the camera of IMPORT-014.1 and an empty import destination
- **When** the camera is imported with `{name}.{ext}`
- **Then** `DSCF0001.RAF` is copied and verified, and the history records the copy with
  the source path `store_00010001/DCIM/100_FUJI/DSCF0001.RAF`

### IMPORT-014.7 – The camera is not changed
- **Given** the camera of IMPORT-014.1
- **When** it is imported
- **Then** its file system holds the same folders and files, with the same content and
  modification times, as before

### IMPORT-014.8 – A second import copies nothing
- **Given** the camera of IMPORT-014.6 after its import
- **When** it is imported again into the same import destination
- **Then** `DSCF0001.RAF` is already imported, and nothing is copied

### IMPORT-014.9 – A camera that goes away fails the remaining files
- **Given** a camera with `DCIM/100_FUJI/DSCF0001.RAF` and `DCIM/100_FUJI/DSCF0002.RAF`
  that stops answering after the first file is copied
- **When** it is imported
- **Then** `DSCF0001.RAF` is copied, `DSCF0002.RAF` fails with the camera's reason, the
  run ends, and it is not ready to be formatted

### IMPORT-014.10 – A finished camera import is ready to be formatted
- **Given** the camera of IMPORT-014.2
- **When** it is imported and every file below `DCIM` is verified at the import
  destination
- **Then** the run is ready to be formatted, whatever lies outside `DCIM`, and its format
  readiness carries the note that only the files below `DCIM` were checked and that the
  files outside `DCIM` are not backed up

### IMPORT-014.11 – A camera without a storage is an error
- **Given** a camera whose file system has no storage folder, as a locked phone or a
  phone set to charge only shows it
- **When** the camera is scanned
- **Then** the scan fails with the error that the camera shows no storage

### IMPORT-014.12 – A DCIM that cannot be looked at keeps the camera from being formatted
- **Given** a camera storage `store_00010001` whose `DCIM` cannot be looked at
- **When** the camera is scanned and imported
- **Then** `store_00010001/DCIM` is left out as "cannot be read", no file is found, and
  the run is not ready to be formatted

## Out of scope

- Finding and opening cameras, and a camera that is in use by another program: the
  `Cameras` port (ADR 0034) and CLI-013.
- Cameras in the guided import, and detecting a camera while Framehive runs (Q1): a
  later spec that generalizes `Device` (ADR 0023).
- Configurable scanned folders and ignore patterns (Q6).
- Deduplicating files across two card slots and marking the slot (Q8).
- iOS devices (Q3).
- Deleting files on the camera after the import.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-06 | Initial version; owner decisions E1–E7 of 2026-11-06 (ADR 0034) |
| 2026-11-06 | IMPORT-014.10: the format readiness of a camera run notes that only the files below `DCIM` were checked (owner decision E8) |
| 2026-11-06 | IMPORT-014.4 covers only a storage without `DCIM`; IMPORT-014.11: a camera without a storage is an error (owner decision E10); IMPORT-014.12: a `DCIM` that cannot be looked at is left out |
