# 0040 – XMP sidecars written by the import

- Status: accepted
- Date: 2026-11-08

## Context

N16 asks the import to write job code, creator, copyright, keywords and rating to XMP
sidecars, as Photo Mechanic does (G6). The owner decided to build this with the whole
of N16 behind one writer, not for the job code alone (#53). ADR 0039 chooses the parser
and serializer; this ADR decides how the import uses them. Sidecars on the source were
so far left out of every import (IMPORT-005.12) and keep the card from being formatted
(IMPORT-008.9). Architecture §8 and ADR 0011 disagreed on the default sidecar name.

The owner decided all points of the milestone on 2026-11-08, each with the recommended
option (D1–D27, requirements §12). This ADR records D2, D4, D11–D24 and D27.

## Decision

### 1. Sidecar naming defaults to the original's full name (D4)

`sidecar_naming` defaults to `original_with_extension` (`DSCF0042.RAF.xmp`); the other
value is `original_without_extension` (`DSCF0042.xmp`, the XMP Part 3 convention).
Reading accepts both. A RAW and its JPEG never share or clash, and darktable and
digiKam read this form; Lightroom and Capture One do not see it for proprietary RAWs.

### 2. Fields and their sources (D11–D15)

The import writes exactly these fields; rating waits for culling (N10) or library
editing (D12):

| Field | Property | Source |
|---|---|---|
| Job code | `photoshop:TransmissionReference` | the import's job code (IMPORT-006) |
| Creator | `dc:creator`, ordered list | `metadata.creator` |
| Copyright | `dc:rights`, `x-default` | `metadata.copyright` |
| Keywords | `dc:subject`, `lr:hierarchicalSubject`, `mwg-kw:Keywords` | `metadata.keywords` |

A global `metadata:` section in `<configuration folder>/config.yaml` sets the values,
and an import profile overrides it key by key (D13). The job code keeps its own path
(`--job-code`, the guided import, profiles). A keyword `Holiday/Beach` becomes
`Holiday|Beach` in `lr:hierarchicalSubject`, the hierarchy `Holiday` › `Beach` in
`mwg-kw:Keywords` and only the leaf `Beach` in `dc:subject`, the mapping of IMMICH-001
(D14). `{year}` in the copyright is the capture year (CORE-005), or the import date's
year from the `Clock` port when a file has no capture time (D15).

### 3. When a sidecar is written (D16, D17)

A photo gets a sidecar only when at least one value is set or a source sidecar comes
with it; never an empty one. Videos get no values in this milestone; a video's source
sidecar is carried along unchanged.

### 4. Source sidecars come with their file (D2)

The scan attaches a source `.xmp` to the file it names by base name, a narrow part of
LIBRARY-001 grouping: `DSCF0042.RAF.xmp` to `DSCF0042.RAF`, `DSCF0042.xmp` to the file
with the base name `DSCF0042` (the RAW, when a RAW and a rendered file share it). The
sidecar is copied with its file and the import's values are merged into it. A source
sidecar without its file stays left out and keeps the card. `.THM` files stay left out
until full grouping.

### 5. Existing values (D18)

When the carried-along sidecar already has a value: keywords are unioned, the import's
job code replaces the old one, and creator and copyright already present are kept.
The summary lists each replaced or kept value.

### 6. Conflicts at the target (D19)

A sidecar shares its file's conflict outcome (IMPORT-003): renamed with the same suffix,
skipped together, or failed together. It is never merged with a sidecar of another
file.

### 7. Writing, verification and the card (D20–D22, D24)

The sidecar is written after the original's verified copy, through the `files` port
as `<name>.part` and a rename. Before the rename, the writer checks that nothing appeared
at, or changed, the target sidecar since it was read; if it did, the sidecar is not
replaced and the file fails. After the rename, the sidecar is read again, parsed and
compared; its BLAKE3 goes into the import history. Each backup destination gets the
same bytes, verified like the original. A sidecar that cannot be written is a per-file
error naming the path, and keeps the card from being formatted. Permissions are never
changed. A write queue for offline and online-only roots (S3) comes with the library.

### 8. Sidecars only (D23)

The milestone writes no embedded XMP. DNG embedding (ADR 0015) and JPEG/HEIF embedding
(5.1) follow as their own changes; until then, Lightroom and Capture One do not see the
values on JPEG and DNG.

### 9. Part of the import job (D27)

Writing sidecars is part of the import job, which already owns destinations,
verification and the summary. A separate job kind for writing XMP comes with library
writes.

## Consequences

- An import without `metadata:` values and without source sidecars behaves as before.
- Cards with camera or phone sidecars can be formatted after an import, once PR 5 of
  the milestone lifts IMPORT-005.12 and IMPORT-008.9.
- `config.yaml` is a new file type with its own JSON Schema; the import profile gets
  `schema_version` 2 for its `metadata` keys.
- Round trips run through exiftool in CI, and checked-in fixtures exported from
  darktable and digiKam prove that their fields survive (D25, D26).
- External changes to sidecars (S6), merges between machines (S8, T13) and Nextcloud
  conflict copies (NC4) stay with the library milestone.

## Alternatives

- **`original_without_extension` as the default (D4 B).** Seen by Lightroom and
  Capture One for RAWs, but a RAW and its JPEG share one sidecar, and darktable writes
  a second one next to it.
- **Asking on the first import (D4 C).** One more prompt for a choice most users cannot
  judge yet.
- **Keeping source sidecars out (D2 B).** A written sidecar would sit next to a left-out
  source sidecar, and the card could never be formatted.
- **More IPTC fields (D11 B, C), a fixed rating (D12 B, C).** More keys and mappings
  before library editing needs them; a rating per import carries little meaning.
- **Values only in profiles (D13 B) or as CLI flags too (D13 C).** Profiles would repeat
  creator and copyright; flags add CLI surface for one-offs.
- **Ancestors in `dc:subject` (D14 B), flat keywords (D14 C).** B differs from
  IMMICH-001; C postpones hierarchies.
- **Always a sidecar (D16 B), or only behind a switch (D16 C).** A file per photo for
  nothing, or one more key.
- **Import wins every field (D18 B), or the existing sidecar wins (D18 C).** B can
  silently replace what a camera or user set; C cannot relabel a job code.
- **Merging into a target sidecar of another file (D19 B).** Mixes metadata of two
  photos.
- **A failed sidecar as a warning only (D20 B, C).** The card could be formatted while
  the user's metadata exists nowhere else.
- **No sidecars in backups (D21 B).** A backup would not restore the metadata.
- **Skipping silently (D22 B), a write queue now (D22 C).** B hides lost metadata; C is
  needed by the library, not the import.
- **Embedding now (D23 B, C).** Changes originals and needs the fault-injection tests of
  ADR 0015.
- **No check before the rename (D24 B, C).** Can silently drop another program's write.
- **A separate write job (D27 B).** Splits one import's outcome over two jobs.
