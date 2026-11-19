# IMPORT-022 – Stall limit for a source

- Status: implemented
- Module: import
- Origin: T5, N26
- Depends on: IMPORT-004, IMPORT-005, IMPORT-011

## Purpose

A card reader, a camera or a phone can stop answering in the middle of an import. A
single file must never block the run (N26), so a source that does not answer within a
time limit fails its file, and the import goes on with the other sources instead of
waiting forever.

## Behaviour

**The limit.** Every call the import makes to a source (listing a folder, reading a
file's metadata, opening a file, reading bytes from it) must return within the stall
limit. Each call that returns starts the limit again, so a slow source that keeps
delivering bytes never fails, however long a file takes; only a source that delivers
nothing for the whole limit does. The limit is 30 seconds unless the profile sets
`source_stall_limit_seconds` (below). Calls to the destination and the backup
destinations have no limit.

**A call that exceeds it.** The import stops waiting for the call. The source is then
*stopped*: the import makes no further call to it, for the rest of the run.

- During the copy of a file (IMPORT-004), that file fails with
  `cannot read the source: <path>: the source did not answer for <n> seconds`, and its
  `.part` file is removed as for any failed file.
- Every remaining file of that source fails at once, without a call to the source,
  with `cannot read the source: <path>: the source stopped answering`.
- During the scan (IMPORT-005), the scan of that source fails with
  `<path>: the source did not answer for <n> seconds`, and no file of it is imported.

The other sources of the run go on as before. A call that never returns cannot be
cancelled; it stays blocked until the process ends, and the run ends without waiting for
it. A stopped source therefore holds at most one blocked call, so the memory of a run
does not grow with the number of files on a hanging source.

The failed files count as failed in the summary and in format readiness (CLI-007), like
any other failed file.

**The profile key.** IMPORT-011 gains one key, which needs `schema_version` 3, because
the keys of a released format change only with a new version (ADR 0032):

| Key | Type | Default | Meaning |
|---|---|---|---|
| `source_stall_limit_seconds` | whole number | `30` | Seconds a source may take to answer one call before its file fails (IMPORT-022) |

Each check is an error at the value, in the form of CORE-006:

- a value that is not a whole number from 1 to 3600:
  `expected a whole number of seconds from 1 to 3600, found <value>`;
- the key in a profile of `schema_version` 1 or 2:
  `source_stall_limit_seconds needs schema_version 3`.

## Acceptance criteria

### IMPORT-022.1 – A hanging read fails the file
- **Given** a source with `DSCF0001.RAF`, `DSCF0002.RAF` and `DSCF0003.RAF`, in that
  order of the import plan, where the first read of `DSCF0002.RAF` during its copy never
  returns, and the default limit
- **When** it is imported
- **Then** `DSCF0001.RAF` is copied, `DSCF0002.RAF` fails with
  `cannot read the source: <path>: the source did not answer for 30 seconds` after the
  limit, and neither `DSCF0002.RAF` nor its `.part` file is at the destination

### IMPORT-022.2 – The remaining files of a stopped source fail at once
- **Given** the source and the hang of IMPORT-022.1
- **When** it is imported
- **Then** `DSCF0003.RAF` fails with
  `cannot read the source: <path>: the source stopped answering`, the import makes no
  call to the source after the hanging one, and the run ends without waiting for that
  call

### IMPORT-022.3 – A slow source that keeps answering is not failed
- **Given** a source with `DSCF0001.RAF` whose every read returns some bytes after 20
  seconds, so the whole copy takes several minutes, and the default limit
- **When** it is imported
- **Then** `DSCF0001.RAF` is copied and verified

### IMPORT-022.4 – Other sources go on
- **Given** two sources, the first with the hang of IMPORT-022.1 and the second with
  `DSCF0101.RAF`
- **When** both are imported in one run
- **Then** the files of the first source fail as in IMPORT-022.1 and .2, and
  `DSCF0101.RAF` is copied

### IMPORT-022.5 – A hanging scan fails the source
- **Given** a source whose listing of the folder `DCIM/100_FUJI` never returns
- **When** it is imported
- **Then** the scan of that source fails with
  `<path>: the source did not answer for 30 seconds`, and no file of it is copied

### IMPORT-022.6 – The profile sets the limit
- **Given** a profile of `schema_version` 3 with `source_stall_limit_seconds: 5`, and the
  hang of IMPORT-022.1
- **When** it is imported with that profile
- **Then** `DSCF0002.RAF` fails after 5 seconds with
  `cannot read the source: <path>: the source did not answer for 5 seconds`

### IMPORT-022.7 – Invalid limits are rejected
- **Given** a profile of `schema_version` 3 with `source_stall_limit_seconds` set to
  `0`, to `3601` and to `2.5`, and a profile of `schema_version` 2 with
  `source_stall_limit_seconds: 30`
- **When** each is read
- **Then** the first three are errors at the value
  `expected a whole number of seconds from 1 to 3600, found <value>`, and the last is an
  error at the value `source_stall_limit_seconds needs schema_version 3`

## Out of scope

- How the import stops waiting for a call (a thread that makes the calls, and how tests
  measure the limit without the wall clock) is decided in ADR 0049.
- The run of T5 against the binary, with the fault `hang-while-reading`, is a criterion
  of CLI-025.
- Calls to the destination and to backup destinations, which have no limit.

## History

| Date | Change |
|---|---|
| 2026-11-19 | Initial version, from the owner's decisions of 2026-11-19: a stall limit per call rather than per file, 30 seconds by default and set in the profile, and a source that exceeds it is stopped for the rest of the run |
| 2026-11-19 | Out of scope points to ADR 0049, which decides how the import stops waiting |
| 2026-11-19 | Implemented: the import job and `framehive import` read the source through a stall guard; profile `schema_version` 3. The scan and plan of the guided import before its job are not guarded yet (TODO.md) |
