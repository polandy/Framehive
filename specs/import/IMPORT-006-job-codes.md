# IMPORT-006 – Job codes

- Status: implemented
- Module: import
- Origin: R9
- Depends on: IMPORT-001

## Purpose

Photographers sort shoots by what they were for: a wedding, a client, a trip. A job code
is a short label the user gives an import, so that names and folders can carry it
through the `{job}` token of a name template (IMPORT-001).

## Behaviour

A job code is the text the user supplies, with leading and trailing whitespace removed.
Whitespace is what Unicode calls White_Space; whitespace inside the text is kept.

A job code is valid when, after trimming, it

- has at least 1 and at most 64 characters (Unicode scalar values, not bytes),
- contains no `/`, and
- contains no control character (Unicode category Cc: U+0000–U+001F and
  U+007F–U+009F).

Otherwise it is rejected with an error that names the broken rule. Validation is pure:
it does not depend on the file system, the language setting or the time.

**Using it.** An import has at most one job code. The `{job}` token inserts it into the
rendered path, and compatibility mode (IMPORT-001) applies to it like to every other
value. A job code that renders to an invalid segment, such as `..`, makes those files
not nameable (IMPORT-001.18).

**Missing job code.** When the name template contains `{job}` and the import has no job
code, the import does not start: nothing is scanned or copied, and the error names
`{job}`.

An import with a job code whose template has no `{job}` runs as usual; the job code does
not change any name. How the CLI reports this: CLI-001.

## Acceptance criteria

### IMPORT-006.1 – Surrounding whitespace is removed
- **Given** the text `"  Smith wedding \n"`
- **When** it is taken as a job code
- **Then** the job code is `Smith wedding`

### IMPORT-006.2 – At most 64 characters
- **Given** a text of 64 times `ü` (128 bytes), and a text of 65 times `a`
- **When** each is taken as a job code
- **Then** the first is accepted, and the second is rejected with an error that names
  the limit of 64 characters

### IMPORT-006.3 – Empty text
- **Given** the texts `""` and `"   "`
- **When** each is taken as a job code
- **Then** each is rejected with an error that says the job code is empty

### IMPORT-006.4 – No slash
- **Given** the text `Smith/Jones`
- **When** it is taken as a job code
- **Then** it is rejected with an error that names `/`

### IMPORT-006.5 – No control characters
- **Given** the texts `"Smith\tJones"` and `"Smith\u{85}Jones"`
- **When** each is taken as a job code
- **Then** each is rejected with an error that says it contains a control character

### IMPORT-006.6 – A template with `{job}` needs a job code
- **Given** the template `{job}/{name}.{ext}` and an import without a job code
- **When** the import run starts
- **Then** it stops with an error that names `{job}`, and no file is scanned or copied

### IMPORT-006.7 – A job code without `{job}` changes nothing
- **Given** the template `{name}.{ext}`, the job code `Smith wedding` and the file
  `DSC_0042.RAF`
- **When** the import runs
- **Then** the file is copied as `DSC_0042.RAF`

## Out of scope

- Remembering the job codes the user has entered, and offering them again: a later
  version of this spec, with the guided import (CLI-002).
- Asking for a job code at the start of an import: the guided import (CLI-002).
- Writing the job code to XMP as IPTC Job Id (`photoshop:TransmissionReference`, N16):
  the XMP milestone.
- Job codes stored in profiles (N8): the profile spec.
- The `--job-code` argument: CLI-001.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-01 | Initial draft, with the rules the owner decided (guided import plan, PR 1) |
| 2026-11-01 | Accepted by the owner |
| 2026-11-01 | Implemented: `JobCode` in the import module, `{job}` in the naming service |
