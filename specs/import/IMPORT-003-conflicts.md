# IMPORT-003 – Name conflicts and files already imported

- Status: accepted
- Module: import
- Origin: I5, Q8, N3
- Depends on: IMPORT-001, IMPORT-004

## Purpose

Running an import twice must not copy a photo twice, and a new photo must never replace
one that is already in the library. When two different files would get the same name,
the user chooses what happens.

## Behaviour

When the import plan is made, each file's target path is checked against the files at
the destination and against the other files of the plan, in scan order (IMPORT-005).

**Identical content.** Two files have identical content when their sizes are equal and
their checksums are equal; the checksum is only computed when the sizes match.

**Files already imported.** When the target path holds a file with identical content,
the file is marked "already imported" and is not copied. This holds for every conflict
policy, so running the same import again copies nothing.

**Name conflicts.** When the target path holds a different file, or a folder, the
conflict policy decides:

| Policy | Effect |
|---|---|
| `add_suffix` (default) | The file gets the first free name of `<base>-1.<ext>`, `<base>-2.<ext>`, …, where `<base>` is the rendered name before its last `.` |
| `skip` | The file is not copied and is marked "skipped: name conflict" |
| `fail` | The file is not copied and is marked as failed: "name conflict" |

With `add_suffix`, every candidate on the way is checked like the target: a candidate
that holds identical content marks the file "already imported", so a re-run after a
suffix was added still copies nothing.

**Within one plan.** A file whose target path was already given to an earlier file of
the same plan is treated the same way: identical content marks it "duplicate of
<earlier file>" and it is not copied, whatever the policy; different content is
resolved by the policy. So the same photo on two cards in two card slots is copied
once.

**Case.** Two target paths that differ only in upper and lower case are the same
target, because memory cards, macOS and SMB shares do not tell them apart. So a plan
gives the same names on every file system.

**Never overwrite.** No policy replaces or changes a file at the destination. A file
that appears at a target path after the plan was made is handled by IMPORT-004.9.

## Acceptance criteria

### IMPORT-003.1 – Running the same import again copies nothing
- **Given** a source that was imported once, unchanged since
- **When** it is imported again with any conflict policy
- **Then** every file is marked "already imported", and no file is copied

### IMPORT-003.2 – Suffix for a different file
- **Given** the target `2026/DSCF0001.RAF` exists with other content, and the policy
  `add_suffix`
- **When** the import plan is made for `DSCF0001.RAF`
- **Then** its target is `2026/DSCF0001-1.RAF`

### IMPORT-003.3 – The first free suffix
- **Given** `DSCF0001.RAF` and `DSCF0001-1.RAF` exist at the target folder with other
  content, and the policy `add_suffix`
- **When** the import plan is made for `DSCF0001.RAF`
- **Then** its target is `DSCF0001-2.RAF`

### IMPORT-003.4 – A re-run after a suffix copies nothing
- **Given** `DSCF0001.RAF` with other content and `DSCF0001-1.RAF` with the source's
  content exist at the target folder, and the policy `add_suffix`
- **When** the import plan is made for `DSCF0001.RAF`
- **Then** the file is marked "already imported"

### IMPORT-003.5 – Skip
- **Given** the target exists with other content, and the policy `skip`
- **When** the file is imported
- **Then** it is not copied, it is marked "skipped: name conflict", and the existing
  file is unchanged

### IMPORT-003.6 – Fail
- **Given** the target exists with other content, and the policy `fail`
- **When** the file is imported
- **Then** it is not copied, it is marked as failed with "name conflict", the existing
  file is unchanged, and the other files are imported

### IMPORT-003.7 – A folder at the target path
- **Given** a folder exists at the target path `2026/DSCF0001.RAF`, and the policy
  `add_suffix`
- **When** the import plan is made
- **Then** the file's target is `2026/DSCF0001-1.RAF`, and the folder is unchanged

### IMPORT-003.8 – Two files of one plan with the same target
- **Given** `a/DSCF0001.JPG` and `b/DSCF0001.JPG` with different content, the same
  capture time, a template that gives both the same target, and the policy `add_suffix`
- **When** the import plan is made
- **Then** `a/DSCF0001.JPG` keeps the target, and `b/DSCF0001.JPG` gets the suffix `-1`

### IMPORT-003.9 – The same photo twice in one source
- **Given** `slot1/DSCF0001.RAF` and `slot2/DSCF0001.RAF` with identical content and
  the same target
- **When** the import plan is made with any policy
- **Then** `slot1/DSCF0001.RAF` is planned, and `slot2/DSCF0001.RAF` is marked
  "duplicate of slot1/DSCF0001.RAF" and not copied

### IMPORT-003.10 – Targets that differ only in case
- **Given** `a/dscf0001.jpg` and `b/DSCF0001.JPG` with different content, a template
  `{name}.{ext}`, and the policy `add_suffix`
- **When** the import plan is made
- **Then** `a/dscf0001.jpg` gets `dscf0001.jpg`, and `b/DSCF0001.JPG` gets
  `DSCF0001-1.JPG`

### IMPORT-003.11 – Sizes decide first
- **Given** the target exists with a different size than the source
- **When** the import plan is made
- **Then** the files are treated as different, and neither is read to compute a
  checksum

## Out of scope

- A file that appears at the target during the copy: IMPORT-004.9.
- Recognizing files already imported after they were renamed or moved, through an
  import history (N3, N6): a later spec.
- Conflicts at backup destinations (I5, overwrite or skip): the backup spec.
- Where the policy is set: CLI-001 for the command line, the profile spec (N8) later.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-10-31 | Initial draft (milestone 1) |
| 2026-10-31 | Accepted by the owner |
