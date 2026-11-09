# CORE-010 – XMP sidecar files

- Status: accepted
- Module: core (metadata service)
- Origin: N16, 5.1, M2, T11
- Depends on: CORE-009

## Purpose

Sidecars are files that other programs write too. Framehive finds them under both
namings, saves its changes without ever replacing what another program wrote meanwhile,
and stays readable for exiftool, darktable and digiKam.

## Behaviour

**Sidecar names.** Two namings exist (ADR 0040): `original_with_extension`
(`DSCF0042.RAF.xmp`, the default) and `original_without_extension` (`DSCF0042.xmp`).
Reading accepts both; the extension `xmp` is matched in any case.

**Saving.** A sidecar is written through the `files` port to `<name>.part` and then
renamed. Before the rename, the writer compares the file at the target with what it
found there before: if a sidecar appeared or its size, modification time or BLAKE3
changed, the target is left as it is and the write fails. A sidecar that cannot be
parsed is never overwritten. Permissions are never changed.

## Acceptance criteria

### CORE-010.1 – A sidecar that cannot be parsed is kept
- **Given** a target sidecar that cannot be parsed
- **When** a value is to be merged into it
- **Then** the write fails with the parse error, and the file is byte for byte as before

### CORE-010.2 – The write is atomic
- **Given** a sidecar being written, and the `files` port failing the rename
- **When** the write ends
- **Then** the target is as before, no `.part` file is left, and the error names the
  path

### CORE-010.3 – A sidecar that appeared is not replaced
- **Given** no sidecar at the target when the write began, and another program
  creating one before the rename
- **When** the write reaches the rename
- **Then** the other program's sidecar is unchanged, and the write fails with an error
  that names the path

### CORE-010.4 – A sidecar changed meanwhile is not replaced
- **Given** a sidecar read for a merge, and its content changed by another program
  before the rename
- **When** the write reaches the rename
- **Then** the changed sidecar is kept, and the write fails with an error that names
  the path

### CORE-010.5 – A read-only sidecar
- **Given** a target sidecar the user cannot write
- **When** a value is merged into it
- **Then** the write fails with an error that names the path, and the file's
  permissions and content are unchanged

### CORE-010.6 – Both sidecar names are read
- **Given** `DSCF0042.RAF` with `DSCF0042.RAF.XMP` next to it, and `DSCF0043.RAF` with
  `DSCF0043.xmp`
- **When** the sidecars of each file are looked up
- **Then** `DSCF0042.RAF.XMP` is found for the first and `DSCF0043.xmp` for the second

### CORE-010.7 – Names follow the naming setting
- **Given** the file `20261108-0042.RAF`
- **When** its sidecar name is made with `original_with_extension` and with
  `original_without_extension`
- **Then** the names are `20261108-0042.RAF.xmp` and `20261108-0042.xmp`

### CORE-010.8 – exiftool reads what Framehive writes
- **Given** a sidecar written with job code, creator, copyright and the keyword
  `Holiday/Beach`
- **When** exiftool reads it
- **Then** it reports the same values and no warning

### CORE-010.9 – darktable sidecars survive
- **Given** the checked-in sidecars that darktable wrote for the DNG and the JPEG of
  `dng/canon-eos-350d/` (CORE-004)
- **When** a keyword is merged into each and it is written
- **Then** reading it again gives every original property, including darktable's
  history, unchanged

### CORE-010.10 – Framehive reads exiftool's sidecars
- **Given** the .22 values, written by exiftool
- **When** Framehive reads them and adds `Holiday/Lisbon`
- **Then** it reads the same; exiftool sees only that keyword and
  `xmp:MetadataDate` as new, no warning

### CORE-010.11 – digiKam sidecars survive
- **Given** checked-in sidecars exported from digiKam
- **When** a keyword is merged into each and it is written
- **Then** reading it again gives every original property, including digiKam's
  `TagsList`, unchanged

## Out of scope

- The XMP model, its parsing, merging and canonical form: CORE-009.
- Which values the import writes, and when: IMPORT-019 and IMPORT-020.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-09 | Split from CORE-009 (.15–.24 became .1–.10); .1–.8 and .10 implemented, .9 (the fixtures of other programs) waits on the owner's darktable and digiKam sidecars |
| 2026-11-09 | .9 split by program: .9 (darktable) implemented with the owner's EOS 350D sidecars, .11 (digiKam) waits on the owner's digiKam sidecars |
