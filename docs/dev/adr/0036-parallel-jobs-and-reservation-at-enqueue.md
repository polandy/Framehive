# 0036 – Parallel jobs and reservation at enqueue

- Status: accepted
- Date: 2026-11-07

## Context

The job queue of ADR 0035 runs one job at a time, so a second card waits for the first
even when both go to separate disks (J3, Q5). Two imports that run at the same time must
still not get in each other's way: two imports to one disk compete for its bandwidth,
two imports from one card or camera compete for the device, and two imports must never
take the same sequence numbers or the same target names.

Today an import reserves its sequence range when it starts (IMPORT-002), so the numbers
of a queued job depend on when it runs, not on when it was enqueued. Conflicts with
files already on disk are resolved file by file while the import runs (IMPORT-003).

## Decision

### 1. One worker thread per running job

The queue keeps its waiting jobs in enqueue order and starts a `std::thread` for each
job it starts. When a job ends, its thread reports the end and starts the jobs that can
start now. Stopping, removing and enqueueing still never wait. Shutting down asks every
running job to stop, removes the waiting ones and joins every worker thread.

### 2. Jobs name opaque resource keys

`Job::resources` returns the job's `ResourceKey`s, read once when the job is enqueued.
The `jobs` service only compares keys for equality and knows nothing about devices or
imports. The default is no resource: such a job conflicts with nothing.

### 3. The scheduling rule

A waiting job starts when it shares no resource key with a running job and none with a
job enqueued before it that still waits. Jobs that share a resource therefore run in
enqueue order, and a later job overtakes an earlier one only when they share nothing.
There is no further limit on parallel jobs, and no configuration key (owner decision
D3).

### 4. The resources of an import job

An import job names `source file system <id>` of its source folder, or
`source camera <name>` for an opened camera, and `destination file system <id>` of the
import destination, the video destination, and each backup and video backup
destination. The id is the `FileSystemId` of the `FreeSpace` port (ADR 0033), which is
`st_dev` on Unix. A folder whose file system the port cannot tell names its path
instead, so it is never wrongly taken for another file system. Sources and destinations
are kept apart, as the owner decided (D2): a source device and a destination file system
only conflict with their own kind.

### 5. Sequence ranges are reserved at enqueue

An import job given its plan reserves the sequence range of the plan when the import
module enqueues it (`ImportJob::enqueue`), and its start does not reserve again. A job
that is removed leaves its numbers as a gap, as a cancelled import already does. A job
without a plan cannot know how many assets it has before it scans, so it reserves when
it starts, as before; both frontends enqueue planned jobs.

### 6. Target names are reserved in the process

The import jobs of one application share a `ReservedTargetNames` register, passed in
`ImportPorts` like the session counter. Enqueueing a planned job reserves the absolute
targets of its planned files at the import destination and the video destination,
compared by their canonical caseless key (ADR 0026). The reservation ends when the job
ends or is removed: the job owns a guard whose `Drop` releases the names. A name that
another job reserved counts like an existing file with other content when a job
resolves its conflicts (IMPORT-003). The register is not persisted and not shared
between processes; there IMPORT-003 alone applies, as before.

With the scheduling rule, two jobs that write to one file system never run at the same
time, so the register matters where a file system cannot be told and two destinations
are named by their paths.

### 7. Waiting jobs can be woken by a stop request

`JobControl::notify_on_stop` registers a function that is called once when the job is
asked to stop, or at once if it already was. An import job that asks its enqueuer
before each file waits on one answer channel, and the stop notification sends `false`
into it, so a queue that shuts down never waits for an enqueuer that holds the reports
but does not follow them.

### 8. `framehive import` names sources and destination as `cp` does

```
framehive import [OPTIONS] <source>... <destination>
framehive import [OPTIONS] --destination <folder> <source>...
```

Without `--destination`, the last positional argument is the destination and every
other one is a source, so at least two are needed; with `--destination`, every
positional argument is a source (CLI-016). A source is a folder or
`camera:[<model>|<port>]`; a camera cannot be the destination. `--destination` has no
short form.

The meaning of a positional argument never depends on `--profile`, so an import
profile no longer names the destination: with a destination in the profile and one on
the command line, `framehive import SD1 SD2 --profile p` could mean "import SD1 and SD2
into the profile's destination" or "import SD1 onto the card SD2", depending on the
profile. This removes the key `destination` of IMPORT-011 and amends ADR 0032; a
profile that still has it stops the command with a message that says where the folder
goes. The profile keeps `schema_version: 1`: there is no release yet, so no profile in
use has the key. `video_destination` and `backup` stay in the profile.

The `cp` model is familiar, and `framehive import <source> <destination>` of the first
version keeps working. The command checks the arguments first, then the profile, then
the folders, and stops before any job at the first problem (CLI-016).

*Superseded on 2026-11-07:* an earlier version of this decision (C) made every
positional argument a source and named the destination only by `--destination` or the
profile, with a hint for the old habit of a last folder as destination. The owner
replaced it with the `cp` model above, which removes the profile's destination instead.

## Alternatives

- **A fixed pool of N worker threads:** a limit the owner did not want (D3), and a
  pool still needs the conflict rule.
- **Resources as typed values (`Device`, `FileSystem`) in `jobs`:** makes the service
  know about imports; opaque keys keep it generic.
- **Reserve names on disk with placeholder files:** visible to other programs and left
  behind by a crash; the in-process register covers parallel jobs without touching the
  disk.
- **Plan every job inside `enqueue`:** would scan on the caller's thread for jobs that
  were meant to scan in the background; the frontends already plan first.
- **The last folder as destination, except with `--profile` and a single folder (D5):**
  whether a folder is a source then depended on the profile (§8).
- **Every positional argument a source, the destination by option or profile (C):**
  keeps the profile's destination but breaks `framehive import <source> <destination>`
  of the first version (§8, superseded).

## Consequences

- Imports on separate devices run in parallel, and the CLI imports several sources with
  one command (CLI-001). `--destination` is new, and import profiles lose `destination`,
  a breaking change of the profile (§8).
- A planned job's numbers are fixed at enqueue. A dry run of several sources reserves
  in a dry-run memory instead (IMPORT-016): its numbers and target names continue
  across the sources as in a real run, and nothing persisted is written.
- A job that panics still fails alone; the worker threads of other jobs go on.
- The guided import queueing a newly inserted card while a job runs is a later step of
  J2.
