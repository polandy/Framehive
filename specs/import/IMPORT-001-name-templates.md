# IMPORT-001 – Name templates

- Status: accepted
- Module: import
- Origin: R1, R6, R8, N9
- Depends on: –

## Purpose

Photographers want imported files sorted into folders and named by when they were
taken, so that a library stays ordered without manual work. A name template describes
both the folders and the file name in one line of text.

## Behaviour

A name template is text that is rendered once per asset file. Its result is a path
relative to the import destination. `/` separates folders; the last segment is the
file name. Everything outside braces is literal text. `{{` and `}}` stand for a literal
`{` and `}`.

Example: `{date:%Y}/{date:%Y-%m-%d}/{date:%Y%m%d}-{name}.{ext}`

| Token | Value |
|---|---|
| `{date:<format>}` | Capture time of the asset |
| `{imported:<format>}` | Time the import was queued, in the machine's local time |
| `{name}`, `{name:lower}`, `{name:upper}` | Original base name, without extension |
| `{ext}`, `{ext:lower}`, `{ext:upper}` | Original extension, without the dot |
| `{number}` | Image number: the trailing digits of the original base name |
| `{number:N}` | The last N digits of the image number, padded with zeros to N (1–6) |
| `{seq_today:N}`, `{seq_stored:N}`, `{seq_session:N}`, `{seq_letters}` | Sequence number (IMPORT-002) |

**Date formats.** A format is literal text with these codes. They are independent of
the language setting, so the same template gives the same names on every machine.

| Code | Value | Example |
|---|---|---|
| `%Y` | Year, four digits | `2026` |
| `%y` | Year, two digits | `26` |
| `%m` | Month, two digits | `10` |
| `%d` | Day of the month, two digits | `07` |
| `%j` | Day of the year, three digits | `280` |
| `%H` | Hour, 00–23 | `14` |
| `%M` | Minute | `05` |
| `%S` | Second | `09` |
| `%%` | A literal `%` | `%` |

A format must not contain `/`, `{` or `}`. Folders come from separate segments.

**Capture time.** The capture time is the wall-clock time the camera recorded, without
conversion to another time zone. Camera time correction (N15) is applied before the
template is rendered. When a file has no capture time, its file modification time is
used, and the import plan marks the file so the user can see it.

**Assets.** All files of one asset (grouping, LIBRARY-001) are rendered from the same
values: the same capture time, image number and sequence number. The capture time comes
from the RAW file if it has one, otherwise from the first other file that has one. So a
RAW and its JPEG always land in the same folder under the same base name. A sidecar
keeps the suffix it had after the base name: `DSC_0042.RAF.xmp` follows the RAF file.
When no file of the asset has a capture time, the same order picks the modification
time: the RAW file's, or the first other file's if there is no RAW.

**Validation.** A template is checked when it is saved, before any file is rendered. A
template is rejected, with the column of the error, when it has:

- an unknown token or an unknown token option,
- an unknown date code, or a date format with `/`,
- an unbalanced `{` or `}`,
- more than one sequence token,
- a last segment that does not end with `.{ext}` (with or without an option), because
  files of one asset must keep distinct names,
- a leading `/`, an empty segment, or a segment that is `.` or `..`.

**Per-file failures.** When `{number}` is used and the original base name does not end
in a digit, that file cannot be named. The same holds when a rendered segment is empty,
or becomes `.` or `..` (also after compatibility mode strips characters), because it
would not stay inside the destination. The import plan marks the file with the reason;
the other files are planned normally.

**Compatibility mode.** On by default. It keeps names valid on memory cards (exFAT),
network shares (SMB) and macOS. In every rendered segment it replaces
`< > : " \ | ? *` and control characters (U+0000–U+001F, U+007F) with `_`, and removes
trailing dots and spaces. When it is off, values are used unchanged; an original name
can never contain `/`.

## Acceptance criteria

### IMPORT-001.1 – Folders and name from the capture time
- **Given** `DSC_0042.RAF` captured at 2026-10-17 14:05:09 and the template
  `{date:%Y}/{date:%Y-%m-%d}/{date:%Y%m%d}-{name}.{ext}`
- **When** the import plan is made
- **Then** the file's path is `2026/2026-10-17/20261017-DSC_0042.RAF`

### IMPORT-001.2 – Literal text and escaped braces
- **Given** `DSC_0042.RAF` and the template `{{draft}}_{name}.{ext}`
- **When** the import plan is made
- **Then** the file's name is `{draft}_DSC_0042.RAF`

### IMPORT-001.3 – Case options
- **Given** `DSC_0042.RAF` and the template `{name:lower}.{ext:upper}`
- **When** the import plan is made
- **Then** the file's name is `dsc_0042.RAF`

### IMPORT-001.4 – Image number
- **Given** `DSC_0042.RAF`
- **When** it is rendered with `{number}.{ext}`, `{number:2}.{ext}` and
  `{number:6}.{ext}`
- **Then** the names are `0042.RAF`, `42.RAF` and `000042.RAF`

### IMPORT-001.5 – File without an image number
- **Given** `IMG.JPG` and `DSC_0042.RAF`, and the template `{number}.{ext}`
- **When** the import plan is made
- **Then** `IMG.JPG` is marked as not nameable, the reason names `{number}`, and
  `DSC_0042.RAF` is planned as `0042.RAF`

### IMPORT-001.6 – Import time is fixed when the job is queued
- **Given** an import queued at 2026-10-18 23:59 local time and the template
  `{imported:%Y-%m-%d}/{name}.{ext}`
- **When** its files are copied after midnight
- **Then** every file is in the folder `2026-10-18`

### IMPORT-001.7 – Missing capture time
- **Given** `scan.jpg` without a capture time, modified at 2026-03-01 08:00:00, and the
  template `{date:%Y-%m-%d}/{name}.{ext}`
- **When** the import plan is made
- **Then** the path is `2026-03-01/scan.jpg`, and the plan marks the file as dated by
  its modification time

### IMPORT-001.8 – Files of one asset share their values
- **Given** `DSC_0042.RAF` captured at 2026-10-17 23:59:59, `DSC_0042.JPG` without a
  capture time and modified at 2026-10-18 00:00:01, and the template
  `{date:%Y-%m-%d}/{name}.{ext}`
- **When** the import plan is made
- **Then** both files are in the folder `2026-10-17`

### IMPORT-001.9 – Sidecars follow their file
- **Given** `DSC_0042.RAF`, `DSC_0042.RAF.xmp` and `DSC_0042.xmp` in one folder,
  captured on 2026-10-17, and the template `{date:%Y%m%d}-{number}.{ext}`
- **When** the import plan is made
- **Then** the names are `20261017-0042.RAF`, `20261017-0042.RAF.xmp` and
  `20261017-0042.xmp`

### IMPORT-001.10 – Unknown token
- **Given** the template `{date:%Y}/{camera}.{ext}`
- **When** the user saves it
- **Then** it is rejected, and the error names `{camera}` and column 11

### IMPORT-001.11 – Unknown date code
- **Given** the template `{date:%Y-%B}/{name}.{ext}`
- **When** the user saves it
- **Then** it is rejected, and the error names `%B`

### IMPORT-001.12 – Unbalanced brace
- **Given** the template `photos}/{name}.{ext}`
- **When** the user saves it
- **Then** it is rejected, and the error names column 7

### IMPORT-001.13 – Only one sequence token
- **Given** the template `{seq_today:3}-{seq_session:2}.{ext}`
- **When** the user saves it
- **Then** it is rejected, and the error names `{seq_session:2}`

### IMPORT-001.14 – The extension is required
- **Given** the template `{date:%Y}/{name}`
- **When** the user saves it
- **Then** it is rejected, and the error says the name must end with `.{ext}`

### IMPORT-001.15 – Paths stay inside the destination
- **Given** the templates `/{name}.{ext}`, `a//{name}.{ext}` and `../{name}.{ext}`
- **When** the user saves each of them
- **Then** each is rejected, and the error names the offending segment

### IMPORT-001.16 – Compatibility mode replaces invalid characters
- **Given** compatibility mode is on, `what?.jpg` captured at 14:05, and the template
  `{date:%H:%M} {name}. /{name}.{ext}`
- **When** the import plan is made
- **Then** the path is `14_05 what_/what_.jpg`

### IMPORT-001.17 – Compatibility mode off
- **Given** compatibility mode is off, a file captured at 14:05, and the template
  `{date:%H:%M}/{name}.{ext}`
- **When** the import plan is made
- **Then** the folder is `14:05`

### IMPORT-001.18 – A rendered segment that would leave the destination cannot be named
- **Given** a file whose rendered segment is empty, or becomes `.` or `..` (also after
  compatibility mode strips characters)
- **When** the import plan is made
- **Then** that file cannot be named, and the plan marks it with the reason; the other
  files are planned normally

## Out of scope

- Sequence numbers: IMPORT-002.
- A target name that already exists, or two files with the same target: IMPORT-003.
- Photo and video metadata tokens (R2, R3) and job codes (R9): later specs.
- The live preview and block editor (N9, R10): a UI spec.
- Where a profile stores its template and the mode: the profile spec (N8).

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-10-19 | Initial draft |
| 2026-10-19 | Accepted by the owner |
| 2026-10-19 | Clarified the modification-time fallback order for assets without a capture time, and added IMPORT-001.18 for a rendered segment that would leave the destination (naming service, review) |
