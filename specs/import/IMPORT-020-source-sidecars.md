# IMPORT-020 – Source sidecars

- Status: implemented
- Module: import
- Origin: N16, V7, M2
- Depends on: CORE-009, IMPORT-001, IMPORT-005, IMPORT-008, IMPORT-019

## Purpose

Cameras, phones and other programs leave `.xmp` sidecars on the card. They belong to
their photo and must come along with it, with the import's values merged in, instead of
keeping the card from being formatted forever.

## Behaviour

**Attaching.** The scan (IMPORT-005) attaches a source `.xmp` to the file it
names:

| Source sidecar | Attached to |
|---|---|
| `DSCF0042.RAF.xmp` | `DSCF0042.RAF` |
| `DSCF0042.xmp` | the file with the base name `DSCF0042`; the RAW, if a RAW and a rendered file share it |

The extension `xmp` and the file's extension are matched in any case. A source `.xmp`
without its file is left out with the reason "sidecar without its file" and keeps the
card from being formatted. A `.THM` file is still left out with "sidecar, needs
grouping". A carried-along sidecar keeps its naming form and follows its file's target
name (IMPORT-001.9). The values go into the carried-along sidecar of the configured
naming form, or into the other one when only that came along; a second sidecar is
never written next to it.

**Existing values.** When the carried-along sidecar already has a value:

| Field | Result |
|---|---|
| Keywords | The union of both |
| Job code | The import's job code replaces the old one |
| Creator, copyright | The value already present is kept |

The summary lists each replaced and each kept value with its file.

A carried-along sidecar is written, verified, backed up and recorded as IMPORT-019
describes, and shares its file's conflict outcome. Without values to merge it is
copied byte for byte like any file. It counts as saved at a location only when it was
written and verified there, or, for a file already imported (IMPORT-003, IMPORT-009),
when the sidecar found there has the checksum the import history recorded for it; a
sidecar copied byte for byte also counts with the checksum of its source. The re-run
reads the sidecar at the file's target and at each backup that has the file, and
writes nothing, so a card already saved can be formatted with no extra copying. A
sidecar changed since, or one with values that the history holds no record of, does
not count there.

## Acceptance criteria

### IMPORT-020.1 – A source sidecar comes with its file
- **Given** `DSCF0042.RAF` and `DSCF0042.RAF.xmp` with an unknown namespace, and the
  job code `Lisbon`
- **When** they are imported to `20261017-0042.RAF`
- **Then** `20261017-0042.RAF.xmp` holds every source property plus the job code, no
  other sidecar is written, and both source files count as saved

### IMPORT-020.2 – A sidecar without extension goes to the RAW
- **Given** `DSCF0042.RAF`, `DSCF0042.JPG` and `DSCF0042.xmp`
- **When** they are imported to `20261017-0042.*`
- **Then** `DSCF0042.xmp` becomes `20261017-0042.xmp`, attached to the RAF

### IMPORT-020.3 – A sidecar without its file
- **Given** `DSCF0042.xmp` and no file with the base name `DSCF0042`
- **When** the source is scanned
- **Then** it is left out with the reason "sidecar without its file", and the card
  cannot be formatted

### IMPORT-020.4 – THM files stay left out
- **Given** `clip.MOV` and `clip.THM`
- **When** the source is scanned
- **Then** `clip.THM` is left out with the reason "sidecar, needs grouping"

### IMPORT-020.5 – A video's source sidecar is copied unchanged
- **Given** `clip.MOV`, `clip.MOV.xmp` and the job code `Lisbon`
- **When** they are imported
- **Then** the target `clip.MOV.xmp` has the same bytes as the source

### IMPORT-020.6 – Keywords are unioned
- **Given** a source sidecar with `dc:subject` `Sea`, and the keyword `Beach`
- **When** it is imported
- **Then** `dc:subject` is `Sea`, `Beach`

### IMPORT-020.7 – The job code replaces, creator and copyright are kept
- **Given** a source sidecar with job code `Porto`, creator `Camera Owner` and rights
  `© Camera Owner`, and the import's job code `Lisbon`, creator `Andy Pollari` and
  copyright `© 2026 Andy Pollari`
- **When** it is imported
- **Then** the sidecar has job code `Lisbon`, creator `Camera Owner` and rights
  `© Camera Owner`, and the summary lists the replaced job code and the kept creator and
  copyright for that file

### IMPORT-020.8 – A carried-along sidecar not saved keeps the card
- **Given** `DSCF0042.RAF` with `DSCF0042.RAF.xmp`, and a backup destination failing
  the sidecar
- **When** the format readiness is evaluated with two required locations
- **Then** the card cannot be formatted, and `DSCF0042.RAF.xmp` is listed with one
  location

### IMPORT-020.9 – A re-run counts the sidecar the first run saved
- **Given** `DSCF0042.RAF` with `DSCF0042.RAF.xmp` imported with the job code `Lisbon`
  to the import destination and two backup destinations
- **When** the same import runs again
- **Then** the file is imported before, `DSCF0042.RAF.xmp` counts at three locations
  with no sidecar written, and the card can be formatted

### IMPORT-020.10 – A sidecar changed since does not count there
- **Given** the import of IMPORT-020.9, and the sidecar at one backup destination edited
  since
- **When** the same import runs again
- **Then** `DSCF0042.RAF.xmp` counts at two locations, the edited sidecar is kept, and
  the card cannot be formatted

## Out of scope

- The values, their configuration, conflicts, verification and backups: IMPORT-019.
- `.THM` files and full RAW + JPEG grouping: LIBRARY-001.
- Reading XMP and merging it: CORE-009.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-08 | Created and accepted with the owner's XMP decisions D2 and D18 (ADR 0040 §4, §5); replaces IMPORT-005.12 and IMPORT-008.9 for `.xmp` files when implemented |
| 2026-11-08 | Implemented: the scan attaches each `.xmp` to its file, the run merges the values into it or copies it byte for byte, the summary lists each replaced and kept value, and the format readiness counts it where it was written and verified; IMPORT-005.12 and IMPORT-008.9 updated |
| 2026-11-08 | Owner decision on issue #98 (option 1): a re-run reads the carried sidecar at each location of a file already imported and counts it where it has the recorded checksum (.9, .10 added) |
