# IMPORT-002 – Sequence numbers

- Status: draft
- Module: import
- Origin: R4, R5, J3
- Depends on: IMPORT-001 (file name templates)

## Purpose

Photographers number imported files so that names are unique and sort in shooting
order across cards, cameras and days. The numbering must never produce duplicate names,
even when several imports run at the same time.

## Behavior

A file name template (IMPORT-001) can contain one sequence element. Every sequence kind
produces a number. The number is formatted with a fixed number of digits (1–6, padded
with zeros) or as letters.

| Kind (YAML value) | Template token | Scope | Persisted |
|---|---|---|---|
| `imported_today` | `{seq_today:N}` | All imports on the same calendar day (local time of the machine) | Yes |
| `stored` | `{seq_stored:N}` | All imports ever, per machine | Yes |
| `session` | `{seq_session:N}` | One run of the application | No |
| `letters` | `{seq_letters}` | Like `session`, formatted `a, b, …, z, aa, ab, …, zz` | No |

Each persisted counter has a value per machine and is stored in the machine-local
configuration directory, not in the library.

**Grouping.** Files that belong to the same picture share one number. They have the same
base name, come from the same folder on the same device, and differ only in extension
(for example `DSC_0001.ARW` + `DSC_0001.JPG`, or the `.xmp` sidecar). The number is
assigned once per group.

**Reservation.** Numbers are reserved when a job is *queued*, not when it runs. A queued
job holds a contiguous range for its groups. When a job is cancelled before it copies a
group, its unused numbers are not returned. Gaps are acceptable; duplicates are not.

**Overflow.** If a number no longer fits the configured digits, the job is not queued.
The user sees an error that names the template and suggests more digits.

## Acceptance criteria

### IMPORT-002.1 – Imported-today counter continues within a day
- **Given** the template `{date:%Y%m%d}_{seq_today:3}.{ext}` and an earlier import today that
  used numbers 001–012
- **When** the user queues an import of 3 pictures today
- **Then** the files are named with numbers 013, 014 and 015

### IMPORT-002.2 – Imported-today counter resets on a new day
- **Given** the imported-today counter is at 015 on 2026-10-17
- **When** an import is queued on 2026-10-18 (machine-local time)
- **Then** its first picture gets number 001

### IMPORT-002.3 – RAW and JPEG share a number
- **Given** a card folder with `DSC_0001.ARW`, `DSC_0001.JPG` and `DSC_0002.ARW`
- **When** they are imported with `{seq_session:4}.{ext}`
- **Then** both `DSC_0001` files get `0001` and `DSC_0002.ARW` gets `0002`

### IMPORT-002.4 – Parallel jobs never collide
- **Given** two jobs from different devices queued one after the other, using the same
  `stored` counter, starting at 100
- **When** both run at the same time and finish in any order
- **Then** the first-queued job uses 101…(100+n), the second continues directly after it,
  and no number appears twice

### IMPORT-002.5 – Cancelled job leaves a gap
- **Given** a queued job that reserved 101–110
- **When** it is cancelled before copying, and a new job is queued afterwards
- **Then** the new job starts at 111

### IMPORT-002.6 – Letter sequence
- **Given** the session letter sequence has produced `z`
- **When** the next two pictures are numbered
- **Then** they get `aa` and `ab`

### IMPORT-002.7 – Overflow is rejected when queuing
- **Given** the template `{seq_stored:2}.{ext}` and the stored counter at 98
- **When** the user queues an import of 5 pictures
- **Then** the job is not queued, and the error names the template and suggests
  `{seq_stored:3}`

### IMPORT-002.8 – Counters survive restarts
- **Given** the stored counter is at 250
- **When** the application is restarted and a picture is imported with `{seq_stored:4}.{ext}`
- **Then** it gets `0251`

## Not in scope

- Template syntax and other tokens: IMPORT-001.
- Conflict handling when a target name already exists on disk: IMPORT-003.

## Open points

- Should `letters` continue beyond `zz` (to `aaa`) or overflow like digits? RPD stops at
  `zz`.
- Should persisted counters be shared between machines through the library? Currently
  no, to avoid sync conflicts (ADR 0003).

## History

| Date | Change |
|---|---|
| 2026-10-17 | Initial draft |
| 2026-10-19 | Example templates end with `.{ext}`, as IMPORT-001 requires |
