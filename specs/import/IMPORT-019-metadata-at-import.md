# IMPORT-019 – Metadata at import

- Status: implemented
- Module: import
- Origin: N16, G6, R9, N8, V7
- Depends on: CORE-005, CORE-006, CORE-009, IMPORT-001, IMPORT-003, IMPORT-004,
  IMPORT-005, IMPORT-006, IMPORT-007, IMPORT-008, IMPORT-009, IMPORT-011, IMPORT-013

## Purpose

A photographer wants each photo labelled with job code, creator, copyright and keywords
the moment it is imported, as Photo Mechanic does, so other programs and the library
see them at once.

## Behaviour

**Values.** The import writes these fields (ADR 0040):

| Field | XMP property | Value |
|---|---|---|
| Job code | `photoshop:TransmissionReference` | The import's job code (IMPORT-006) |
| Creator | `dc:creator` (ordered list) | `metadata.creator` |
| Copyright | `dc:rights`, language `x-default` | `metadata.copyright` |
| Keywords | `dc:subject`, `lr:hierarchicalSubject`, `mwg-kw:Keywords` | `metadata.keywords` |

The values come from two configuration files (CORE-006):

- `<configuration folder>/config.yaml`, the file type `configuration`, highest
  `schema_version` 1, with the key `metadata`;
- the import profile (IMPORT-011), which gets `schema_version` 2 with the same key
  `metadata`; a version 1 profile is still read.

| Key | Kind | Default | Meaning |
|---|---|---|---|
| `metadata.creator` | list of text | empty | Creators, in this order |
| `metadata.copyright` | text | none | Copyright notice; `{year}` is replaced |
| `metadata.keywords` | list of text | empty | Keywords; `/` separates the levels of a hierarchy |
| `metadata.sidecar_naming` | text | `original_with_extension` | `original_with_extension` (`DSCF0042.RAF.xmp`) or `original_without_extension` (`DSCF0042.xmp`) |

A key set in the profile replaces the same key of `config.yaml`; keys the profile
leaves out come from `config.yaml`. A file `config.yaml` that does not exist sets
nothing. A keyword that is empty, starts or ends with `/`, or contains `//` or `|` is an
error at its position.

**Keywords.** `Holiday/Beach` is written as `Holiday|Beach` in
`lr:hierarchicalSubject`, as the hierarchy `Holiday` › `Beach` in `mwg-kw:Keywords`, and
as the leaf `Beach` in `dc:subject`.

**`{year}`.** In `metadata.copyright`, `{year}` is the year of the photo's capture time
(CORE-005); a photo without a capture time takes the year of the import date from the
`Clock` port. It is the only token; any other text is written as it is.

**Which files get a sidecar.** A photo gets a sidecar when at least one value is set or
a source sidecar comes with it (IMPORT-020). A video gets no values; its source sidecar is copied
unchanged. No empty sidecar is written. The sidecar's name follows `sidecar_naming`
and the file's target name (CORE-009).

**Conflicts.** A sidecar shares its file's conflict outcome (IMPORT-003): a target is
free only when the file's name and the names of its sidecars are free, a suffix goes to
both, and a skipped or failed file takes its sidecars with it. A file found already at
its target (IMPORT-003) gets its sidecar only where none is there, at the target and
at each backup that holds the file, so a re-run completes a failed one; a sidecar that
is there is never changed. A file the import history knows writes none. With `original_without_extension`, a RAW and its JPEG of the same
base name would share a sidecar name, so the second of them in scan order takes a
suffix: `DSCF0042.JPG` with `DSCF0042.xmp`, then `DSCF0042-1.RAF` with `DSCF0042-1.xmp`.

**Order, verification and backups.** A sidecar is written after its file's verified
copy (IMPORT-004), through `<name>.part` and a rename (CORE-009), then read again and
parsed; the model must equal the one written. Its BLAKE3 is recorded in the import
history (IMPORT-009). Each backup destination (IMPORT-007) gets the same bytes,
verified against them.

**Failure.** A sidecar that cannot be written or verified fails its file, with an error
that names the sidecar path; the file's copy stays in place. A failed file keeps the
card from being formatted (IMPORT-008). A sidecar counts as saved at a location only
when it was written and verified there.

## Acceptance criteria

### IMPORT-019.1 – The values of N16 are written
- **Given** `config.yaml` with creator `Andy Pollari`, copyright `© {year} Andy Pollari`
  and keyword `Holiday/Beach`, the job code `Lisbon`, and `DSCF0042.RAF` captured on
  2026-10-17
- **When** it is imported
- **Then** `DSCF0042.RAF.xmp` next to the target has
  `photoshop:TransmissionReference` `Lisbon`, `dc:creator` `Andy Pollari`, `dc:rights`
  `© 2026 Andy Pollari`, `dc:subject` `Beach`, `lr:hierarchicalSubject`
  `Holiday|Beach` and `mwg-kw:Keywords` `Holiday` › `Beach`

### IMPORT-019.2 – Without values, no sidecar
- **Given** no `metadata` values, no job code and a source without sidecars
- **When** it is imported
- **Then** no sidecar is written, and the result is as before this feature

### IMPORT-019.3 – The profile overrides key by key
- **Given** `config.yaml` with creator `Andy Pollari` and keyword `Family`, and a
  profile with keyword `Holiday`
- **When** a photo is imported with the profile
- **Then** its sidecar has creator `Andy Pollari` and only the keyword `Holiday`

### IMPORT-019.4 – A version 1 profile is still read
- **Given** a profile with `schema_version: 1` and no `metadata` key
- **When** it is read
- **Then** it is accepted, and the values come from `config.yaml`

### IMPORT-019.5 – An invalid keyword
- **Given** a profile with the keyword `Holiday//Beach`
- **When** it is read
- **Then** an error names the file, line and column, and nothing is imported

### IMPORT-019.6 – `{year}` without a capture time
- **Given** copyright `© {year} Andy Pollari`, a photo without a capture time and the
  import date 2026-11-08
- **When** it is imported
- **Then** `dc:rights` is `© 2026 Andy Pollari`

### IMPORT-019.7 – Sidecar naming without extension
- **Given** `sidecar_naming: original_without_extension` and the job code `Lisbon`
- **When** `DSCF0042.RAF` is imported to `20261017-0042.RAF`
- **Then** the sidecar is `20261017-0042.xmp`

### IMPORT-019.8 – A RAW and its JPEG
- **Given** `DSCF0042.RAF` and `DSCF0042.JPG`, the job code `Lisbon` and the default
  naming
- **When** they are imported
- **Then** `DSCF0042.RAF.xmp` and `DSCF0042.JPG.xmp` are written, each with the job code

### IMPORT-019.9 – Videos get no values
- **Given** the job code `Lisbon` and `clip.MOV`
- **When** it is imported
- **Then** no sidecar is written for `clip.MOV`

### IMPORT-019.10 – A suffix goes to the sidecar too
- **Given** `20261017-0042.RAF` with other content at the target, the policy
  `add_suffix` and the job code `Lisbon`
- **When** `DSCF0042.RAF` is imported
- **Then** the file is `20261017-0042-1.RAF` and its sidecar `20261017-0042-1.RAF.xmp`

### IMPORT-019.11 – A leftover sidecar at the target is not merged
- **Given** `20261017-0042.RAF.xmp` at the target without its file, the policy
  `add_suffix` and the job code `Lisbon`
- **When** `DSCF0042.RAF` is imported
- **Then** the leftover sidecar is unchanged, and the file and its sidecar take the
  suffix `-1`

### IMPORT-019.12 – A skipped file writes no sidecar
- **Given** other content at the target, the policy `skip` and the job code `Lisbon`
- **When** the file is imported
- **Then** it is skipped, and no sidecar is written for it

### IMPORT-019.13 – A file already imported keeps its sidecar
- **Given** the same import run again with the job code `Lisbon`, and the sidecar edited
  since
- **When** it runs
- **Then** every file is already imported, and no sidecar is written or changed

### IMPORT-019.14 – Backups get the same sidecar
- **Given** two backup destinations and the job code `Lisbon`
- **When** `DSCF0042.RAF` is imported
- **Then** each backup destination has the sidecar with the same bytes as the import
  destination, and the summary counts it as backed up

### IMPORT-019.15 – The sidecar is recorded
- **Given** the job code `Lisbon`
- **When** `DSCF0042.RAF` is imported
- **Then** the import history records the sidecar with its target and its BLAKE3 at
  every location

### IMPORT-019.16 – A failed sidecar fails its file
- **Given** the `files` port failing the rename of the sidecar
- **When** `DSCF0042.RAF` is imported with the job code `Lisbon`
- **Then** the file fails with an error naming the sidecar path, the copied RAF stays,
  and the card cannot be formatted

### IMPORT-019.17 – A dry run writes nothing
- **Given** the job code `Lisbon` and a dry run
- **When** it runs
- **Then** no sidecar is written, and the plan lists each sidecar it would write

### IMPORT-019.18 – A re-run completes a failed sidecar
- **Given** `DSCF0042.RAF` imported with the job code `Lisbon` and its sidecar failed
  (IMPORT-019.16)
- **When** the same import runs again and the sidecar can be written
- **Then** the file is already imported, its sidecar is written, verified and recorded,
  and the card can be formatted

## Out of scope

- Reading, merging and writing XMP: CORE-009.
- Source sidecars on the card, and values already in them: IMPORT-020.
- Rating and pick state at import (N16 rating, D12): culling (N10) and library editing.
- Further IPTC fields (rights marked, usage terms, contact info): library editing (B9).
- Embedded XMP in DNG, JPEG or HEIF: ADR 0015 and later changes.
- Values for videos, and `.THM` files: library video support and full grouping
  (LIBRARY-001).
- The CLI output of the summary and the dry run, and `framehive config init` writing the
  `metadata` section: the CLI specs.
- A write queue for offline and online-only roots (S3): the library.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-08 | Created and accepted with the owner's XMP decisions D4, D11–D17, D19–D22, D27 (ADR 0040) |
| 2026-11-08 | Implemented: the import writes, verifies, backs up and records the sidecars. The suffix examples of .10 and .11 corrected to `-1`; a RAW and its JPEG that would share a sidecar name under `original_without_extension` follow the suffix rule |
| 2026-11-08 | A re-run completes a failed sidecar (.13 reworded, .18 added) |
