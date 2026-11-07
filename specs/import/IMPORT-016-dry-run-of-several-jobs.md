# IMPORT-016 – A dry run of several import jobs

- Status: implemented
- Module: import
- Origin: Q5, J3
- Depends on: IMPORT-002, IMPORT-003, IMPORT-015

## Purpose

A dry run shows what an import would do. When one command imports several sources, a
real run gives each source's job the numbers and target names after those of the jobs
before it (IMPORT-015, "Enqueueing"). A dry run that reserves nothing would show every
source as if it were imported alone: the same numbers twice, and two files at one
target. The preview must match the real run, and still write nothing that outlives it.

## Behaviour

**Dry-run memory.** The application makes one dry-run memory for the dry run of one
command and gives it to each dry-run import job, and plans each job with its counters.
The memory starts from the persisted sequence counters (IMPORT-002) and keeps every
change to them in memory; the persisted counters are read, never written.

**Reserving in memory.** An import job given its plan and the dry-run memory reserves
at enqueue as a real job does (IMPORT-015, "Enqueueing"): its sequence range from the
memory's counters, session numbers from the session counter, and its target names in
the register of reserved target names. When the job ends, its target names stay
reserved in the memory, not released, because a dry run creates no file that would
keep the next job off them. They are released when the memory is dropped.

**Without the memory.** A dry-run job without the dry-run memory, and a dry run of an
import run, reserve nothing (IMPORT-002, "Reservation").

The guided import has no dry run; its plan summary (CLI-003) is unchanged.

## Acceptance criteria

### IMPORT-016.1 – Dry-run jobs continue each other's numbers
- **Given** the stored counter at 100, a source with three files and a source with two,
  and one dry-run memory
- **When** a dry-run job of each source is planned and enqueued with the memory, one
  after the other, with the template `{seq_stored:3}.{ext}`
- **Then** the first job would copy to `101`, `102` and `103`, the second to `104` and
  `105`, and the stored counter is still 100

### IMPORT-016.2 – A target name of an ended dry-run job stays taken
- **Given** two sources, each with `a.jpg`, the template `{name}.{ext}` and one dry-run
  memory
- **When** the dry-run job of the first source is enqueued and ends, and then the job of
  the second is planned and enqueued
- **Then** the first would copy to `a.jpg`, the second to `a-1.jpg`, and the destination
  has no file

## Out of scope

- A dry run in the guided import.
- Imports in other processes: a dry run shows the numbers as if no other import started
  in between (IMPORT-002).

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-07 | Created and implemented (Q5, J3, ADR 0036): the owner asked that a dry run of several sources counts in memory |
