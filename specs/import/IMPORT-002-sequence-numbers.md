# IMPORT-002 – Sequence numbers

- Status: implemented
- Module: import
- Origin: R4, R5, J3
- Depends on: IMPORT-001 (file name templates), IMPORT-003 (conflicts)

## Purpose

Photographers number imported files so that names are unique and sort in shooting
order across cards, cameras and days. The numbering must never produce duplicate names,
even when several imports run at the same time.

## Behaviour

A file name template (IMPORT-001) can contain one sequence token. Every sequence kind
produces a number, starting at 1. The number is formatted with a fixed number of digits
(1–6, padded with zeros) or as letters.

| Kind (YAML value) | Template token | Scope | Persisted |
|---|---|---|---|
| `imported_today` | `{seq_today:N}` | All imports on the same calendar day (local time of the machine) | Yes |
| `stored` | `{seq_stored:N}` | All imports ever, per machine | Yes |
| `session` | `{seq_session:N}` | One run of the application | No |
| `letters` | `{seq_letters}` | Like `session`, formatted `a, b, …, z, aa, ab, …, zz` | No |

Each persisted counter has a value per machine. It is kept in Framehive's state folder,
not in the configuration and not in the library, so machines never share it (ADR 0029).
The imported-today counter belongs to the machine-local date on which the import starts;
the capture date does not matter.

**Assets.** All files of one asset share one number: the import plan numbers assets, not
files. Which files form an asset is decided by the grouping (IMPORT-001, LIBRARY-001),
for example `DSC_0001.ARW` + `DSC_0001.JPG` + `DSC_0001.ARW.xmp`.

**Order.** The assets of one import are numbered in shooting order: by the capture time
of the asset (IMPORT-001), earliest first. Assets with the same time are ordered by base
name, compared byte by byte.

**Letters.** The letter sequence counts `a`…`z`, then `aa`…`zz`: 702 values. Letter
names sort in shooting order only within one length, because `z` sorts after `aa`.

**Reservation.** Numbers are reserved when an import starts, before its first file is
copied, and an import holds one contiguous range for all its assets. In the guided import
this is Enter; `framehive import` reserves before copying. A dry run (CLI-001) and the
plan summary of the guided import (CLI-003) show the numbers the import would get if no
other import started in between, and reserve nothing. Two imports that start at the same
time, in two processes, get ranges one after the other. When an import is cancelled or
fails, its unused numbers are not returned, and an asset that is not copied (IMPORT-003)
keeps its number unused. Gaps are acceptable; duplicates are not. Session and letter numbers start at 1 in every run and
are never stored.

**Overflow.** If the last number of the range no longer fits the configured digits, or
goes beyond `zz`, the import does not start and reserves nothing. The error names the
template and suggests the template with one more digit; for letters, it suggests
`{seq_session:N}` with enough digits.

**Broken counters.** When the counter file is broken, an import whose template has a
persisted counter does not start; the error names the file and the line (ADR 0029). The
file is not changed. A template without a persisted counter does not read the file.

**Files already imported.** A persisted counter gives the same files a new number on
every import, so their target paths cannot recognize them. The import history (IMPORT-009)
recognizes them instead: a file imported before is left out of the import plan, gets no
sequence number and is not copied. IMPORT-003 still recognizes a file already imported by
its target path when the history does not know it, for example in an import destination
filled on another machine; session and letter numbers start at 1 in every run, so the same
card gets the same names again there.

## Acceptance criteria

### IMPORT-002.1 – Imported-today counter continues within a day
- **Given** the template `{date:%Y%m%d}_{seq_today:3}.{ext}` and an earlier import today that
  used numbers 001–012
- **When** the user starts an import of 3 pictures today
- **Then** the files are named with numbers 013, 014 and 015

### IMPORT-002.2 – Imported-today counter resets on a new day
- **Given** the imported-today counter is at 015 on 2026-10-17
- **When** an import starts on 2026-10-18 (machine-local time)
- **Then** its first picture gets number 001

### IMPORT-002.3 – RAW and JPEG share a number
- **Given** a card folder with `DSC_0001.ARW`, `DSC_0001.JPG` and `DSC_0002.ARW`
- **When** they are imported with `{seq_session:4}.{ext}`
- **Then** both `DSC_0001` files get `0001` and `DSC_0002.ARW` gets `0002`

### IMPORT-002.4 – Parallel imports never collide
- **Given** two imports from different cards, in two processes, started one after the
  other, using the same `stored` counter at 100, the first with n pictures
- **When** both run at the same time and finish in any order
- **Then** the first-started import uses 101…(100+n), the second continues directly after
  it, and no number appears twice

### IMPORT-002.5 – Cancelled import leaves a gap
- **Given** an import that reserved 101–110
- **When** it is cancelled before copying, and a new import starts afterwards
- **Then** the new import starts at 111

### IMPORT-002.6 – Letter sequence
- **Given** the session letter sequence has produced `z`
- **When** the next two pictures are numbered
- **Then** they get `aa` and `ab`

### IMPORT-002.7 – Overflow is rejected before the import starts
- **Given** the template `{seq_stored:2}.{ext}` and the stored counter at 98
- **When** the user starts an import of 5 pictures
- **Then** the import does not start, the stored counter stays at 98, and the error
  names the template and suggests `{seq_stored:3}.{ext}`

### IMPORT-002.8 – Counters survive restarts
- **Given** the stored counter is at 250
- **When** the application is restarted and a picture is imported with `{seq_stored:4}.{ext}`
- **Then** it gets `0251`

### IMPORT-002.9 – A dry run reserves nothing
- **Given** the template `{seq_stored:4}.{ext}` and the stored counter at 250
- **When** the user makes a dry run of 3 pictures, and then starts the import
- **Then** the dry run shows `0251`–`0253`, the import gets `0251`–`0253`, and the
  counter is at 253

### IMPORT-002.10 – A broken counter file stops the import
- **Given** a counter file with the line `stored_last=many`
- **When** the user starts an import with `{seq_stored:4}.{ext}`
- **Then** the import does not start, the error names the file and the line, and the
  file is unchanged

### IMPORT-002.11 – A template without a persisted counter ignores the counter file
- **Given** a broken counter file
- **When** the user imports with `{seq_session:4}.{ext}`
- **Then** the import runs, and the file is unchanged

### IMPORT-002.12 – Shooting order
- **Given** `B.RAF` taken at 10:00:00, `A.RAF` and `C.RAF` both taken at 10:00:05
- **When** they are imported with `{seq_session:1}.{ext}`
- **Then** `B.RAF` gets `1`, `A.RAF` gets `2` and `C.RAF` gets `3`

### IMPORT-002.13 – Letters end at `zz`
- **Given** the template `{seq_letters}.{ext}`
- **When** the user starts an import of 703 pictures
- **Then** the import does not start, and the error names the template and suggests
  `{seq_session:3}.{ext}`

### IMPORT-002.14 – A session counter still recognizes files already imported
- **Given** a card imported with `{date:%Y%m%d}_{seq_session:3}.{ext}` into an import
  destination, and an import history that does not know it
- **When** the same card is imported again with the same template
- **Then** every file gets the number it got before, is marked "already imported"
  (IMPORT-003), and no file is copied

### IMPORT-002.15 – A persisted counter recognizes files imported before
- **Given** a card imported with `{seq_today:3}.{ext}`
- **When** the same card is imported again with the same template, on the same machine
- **Then** every file is imported before (IMPORT-009), no number is reserved, and no file
  is copied

## Out of scope

- Template syntax and other tokens: IMPORT-001.
- Conflict handling when a target name already exists on disk: IMPORT-003.
- Recognizing files already imported by their content: the import history
  (IMPORT-009).
- Sharing counters between machines: not planned, to avoid sync conflicts (ADR 0003,
  ADR 0029).
- Letters beyond `zz`: not planned; RPD stops there too, and longer letter names would
  not sort.

## Open questions


## History

| Date | Change |
|---|---|
| 2026-10-17 | Initial draft |
| 2026-10-19 | Example templates end with `.{ext}`, as IMPORT-001 requires |
| 2026-11-04 | Accepted: counters in the state folder (ADR 0029); reserved when an import starts, as there is no queue yet; assets from the grouping, numbered in shooting order; letters end at `zz`; broken counter file stops the import; notice that persisted counters cannot recognize files already imported; criteria 9–15 |
| 2026-11-04 | Implemented in `naming`, `import`, `adapter-sequence-counters` and `framehive-cli` (CLI-003, CLI-004) |
| 2026-11-04 | The import history recognizes files imported before (IMPORT-009): the plan notice is removed; criterion 15 now tests the recognition, and criterion 14 an import destination the history does not know |
