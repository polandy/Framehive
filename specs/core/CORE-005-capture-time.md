# CORE-005 – Capture time

- Status: implemented
- Module: core (metadata service)
- Origin: F2, N23
- Depends on: –

## Purpose

Photos are sorted, named and found by when they were taken. The capture time is read
from the file's EXIF data, and a file without one is dated by its modification time, so
every file can be named on import.

## Behaviour

The capture time is the wall-clock time the camera recorded: EXIF `DateTimeOriginal`
(CIPA DC-008), with `SubSecTimeOriginal` for fractions of a second and
`OffsetTimeOriginal` for the UTC offset, when the camera wrote them. It is not converted
to another time zone (IMPORT-001).

The EXIF data is found as follows (ADR 0021):

| File | Where the EXIF is |
|---|---|
| JPEG, MPO | The first `APP1` segment with the `Exif` header |
| TIFF-based files: DNG, CR2, NEF, NRW, ARW, SR2, PEF, ORF, RW2, 3FR, DCR, FFF, IIQ, MEF, MOS, SRW, TIFF | The file's own IFDs, in either byte order |
| RAF | The embedded JPEG |

Only `DateTimeOriginal` gives a capture time. The EXIF `DateTime` tag records when the
file was last changed, and `DateTimeDigitized` when it was digitized; neither is used.

A file has no capture time when:

- its format is not in the table (for example videos, CR3, HEIF, PNG) ("format not
  read"),
- it has no `DateTimeOriginal` ("no capture time"),
- the value is blank or all zeros, which EXIF uses for "unknown" ("no capture time"),
- the value is not a valid date and time ("invalid capture time: <value>"),
- the file is damaged, for example truncated or with pointers outside the file or in a
  loop ("damaged: <what>").

Such a file is dated by its modification time, which the file system stores as an
instant in UTC. It is shown in the machine's local time zone, with the offset that was
in force at that instant, including daylight saving time. The reason is reported with
the file.

Invalid sub-seconds or offsets are ignored; the date and time are still used.

## Acceptance criteria

### CORE-005.1 – Capture time from a JPEG
- **Given** a JPEG with `DateTimeOriginal` `2026:10:17 14:05:09`
- **When** its capture time is read
- **Then** it is 2026-10-17 14:05:09, without sub-seconds and without an offset

### CORE-005.2 – Sub-seconds and offset
- **Given** a JPEG with `DateTimeOriginal` `2026:10:17 14:05:09`,
  `SubSecTimeOriginal` `42` and `OffsetTimeOriginal` `+02:00`
- **When** its capture time is read
- **Then** it is 2026-10-17 14:05:09.42 with the offset +02:00, and a name template
  renders `{date:%H%M%S}` as `140509`

### CORE-005.3 – TIFF-based RAW in both byte orders
- **Given** a TIFF-based file in little-endian (`II`) and one in big-endian (`MM`)
  byte order, both with `DateTimeOriginal` `2026:10:17 14:05:09` in the EXIF IFD
- **When** their capture times are read
- **Then** both are 2026-10-17 14:05:09

### CORE-005.4 – RAF through its embedded JPEG
- **Given** a RAF whose embedded JPEG has `DateTimeOriginal` `2026:10:17 14:05:09`
- **When** its capture time is read
- **Then** it is 2026-10-17 14:05:09

### CORE-005.5 – `DateTime` is not a capture time
- **Given** a JPEG with only the EXIF tag `DateTime` `2026:10:18 09:00:00`
- **When** its capture time is read
- **Then** it has none, with the reason "no capture time"

### CORE-005.6 – Unknown values
- **Given** JPEGs whose `DateTimeOriginal` is `0000:00:00 00:00:00`, or 19 spaces
- **When** their capture times are read
- **Then** each has none, with the reason "no capture time"

### CORE-005.7 – Invalid values
- **Given** a JPEG with `DateTimeOriginal` `2026:13:40 25:00:00`
- **When** its capture time is read
- **Then** it has none, and the reason names the value

### CORE-005.8 – Invalid sub-seconds or offset are ignored
- **Given** a JPEG with `DateTimeOriginal` `2026:10:17 14:05:09`,
  `SubSecTimeOriginal` `4x` and `OffsetTimeOriginal` `+25:00`
- **When** its capture time is read
- **Then** it is 2026-10-17 14:05:09, without sub-seconds and without an offset

### CORE-005.9 – Damaged files
- **Given** a JPEG cut off inside its EXIF data, a TIFF whose IFD points outside the
  file, and a TIFF whose IFD points to itself
- **When** their capture times are read
- **Then** each has none, with a reason starting with "damaged", and reading ends
  without a crash

### CORE-005.10 – Formats that are not read
- **Given** `clip.MOV`, `IMG_0001.CR3` and `photo.HIF`
- **When** their capture times are read
- **Then** each has none, with the reason "format not read", and their content is not
  read

### CORE-005.11 – Modification time in local time
- **Given** the machine's time zone is `Europe/Zurich`, and files modified at
  2026-07-01 12:00:00 UTC and 2026-01-01 12:00:00 UTC, without a capture time
- **When** they are dated
- **Then** they are dated 2026-07-01 14:00:00 and 2026-01-01 13:00:00, by their
  modification time

### CORE-005.12 – A RAW file is not read in full
- **Given** a 50 MB TIFF-based RAW whose EXIF IFD is near the start
- **When** its capture time is read
- **Then** at most 1 MB of it is read

## Out of scope

- Other EXIF fields, maker notes and GPS: CORE-001, CORE-003, CORE-004.
- Capture times from video containers, CR3, HEIF and the other formats not in the
  table: later specs.
- Capture times from XMP (sidecar or embedded) and their reconciliation with EXIF per
  MWG: the metadata spec.
- Camera time correction (N15): its own spec.
- Using the capture time in names: IMPORT-001.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-10-31 | Initial draft (milestone 1) |
| 2026-10-31 | Accepted by the owner |
| 2026-10-31 | Implemented in the `metadata` service, with `LocalDateTime` and `CaptureTime` in `domain` |
