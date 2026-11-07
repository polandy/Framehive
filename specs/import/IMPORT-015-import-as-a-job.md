# IMPORT-015 – An import as a job

- Status: implemented
- Module: import
- Origin: J1, J3, Q5
- Depends on: CORE-007, IMPORT-002, IMPORT-003, IMPORT-004, IMPORT-009, IMPORT-014

## Purpose

An import runs as a job in the job queue (CORE-007), so the frontend that started it
stays free: it can show the progress, watch the devices and stop the import while the
files are copied (ADR 0035).

## Behaviour

An **import job** carries everything an import run needs: its settings, the source and
the adapters it reads and writes through, including an opened camera (IMPORT-014). It
runs on the queue's worker thread and does exactly what an import run does on the
caller's thread: the same checks, scan, plan, copies, backups and import history
(IMPORT-004 to IMPORT-014).

**Enqueueing.** An import job is enqueued through the import module, which reserves
before it enqueues: an import job given its plan (see "Starting from a plan") reserves
the sequence range of its plan (IMPORT-002, "Reservation") and the target names of its
planned files at the import destination and the video destination. A job that cannot
reserve its numbers is not enqueued; the enqueuer gets the run error instead. An import
job without a plan, and a dry run, reserve nothing when they are enqueued: the job
reserves its numbers when it starts, as an import run does.

**Reserved target names.** The import jobs of one application share one register of
reserved target names. A name is reserved from the enqueueing until its job ends or is
removed. When an import job resolves its conflicts (IMPORT-003), a target name that
another import job reserved counts like an existing file with other content: under
`add_suffix` the file gets the next free suffix. The register lives in the process;
between processes, IMPORT-003 applies as before.

**Resources.** An import job names as its resources (CORE-007) its source device and
the file systems of its destinations: the file system of the source folder, or the
opened camera; and the file system of the import destination, the video destination,
each backup destination and each video backup destination. The file system is the one
the free space port names (IMPORT-012). A folder whose file system cannot be told names
its path instead. So two import jobs run in parallel only when they share neither a
source device nor a destination file system; otherwise the later one waits.

**Reports.** The import job reports to whoever enqueued it, over a channel of its own,
in this order:

1. the scan of the source, once it is finished;
2. for each file, that it started and its file report once it is done;
3. at the end, the import summary, or the run error when the run could not start.

With the end report, the job hands back an opened camera it was given, so the enqueuer
can release it (CLI-014).

**Job state.** An import that ran, also with failed files, ends as `Finished`. An
import that could not start, for example because the template uses `{job}` without a
job code or a destination has too little space, ends as `Failed` with the run error's
message, and the run error itself is in its end report.

**Stopping.** Asking the import job to stop is the "Stopping" of IMPORT-004: the file
being copied is finished first, the remaining files are not started, the summary says
the run was stopped, and the end of the run is recorded in the import history. The job
ends as `Stopped`. An import job that is removed before it runs does not look at the
source and reports nothing.

**Starting from a plan.** An application that scans, plans and checks free space on its
own thread, to show the plan or a shortage first, can hand the planned run to the import
job instead of the settings alone. The job then starts and executes that plan without
scanning the source again, and reports no scan.

**Asking before each file.** An import job can ask its enqueuer before each file
whether to go on, as a progress view asks during an import run, and waits for the
answer. The enqueuer must follow the job's reports to answer; the guided import uses
the question to look at its devices (CLI-014). An import that its enqueuer's answer
stops is stopped as above and ends as `Stopped`. A job that waits for an answer and is
asked to stop, for example because the queue shuts down, does not wait on: it stops as
above. An enqueuer that dropped the reports cannot answer; the job goes on. An enqueuer
that holds the reports must follow them, or the job waits for its answer until it is
asked to stop.

## Acceptance criteria

### IMPORT-015.1 – A job imports like a run
- **Given** a source with three files, and the same settings for an import run and an
  import job on two equal destinations
- **When** the run is executed and the job runs in a queue
- **Then** both copy the same files to the same target paths, and both summaries count
  the same

### IMPORT-015.2 – Reports in order
- **Given** an import job for a source with two files
- **When** it runs
- **Then** its enqueuer receives the scan, then "started" and the file report for the
  first file, then for the second file, then the summary, in this order; the job ends
  as `Finished`

### IMPORT-015.3 – An import that cannot start fails
- **Given** an import job whose template uses `{job}` and which has no job code
- **When** it runs
- **Then** its end report is that run error, nothing is copied, and the job ends as
  `Failed` with the error's message

### IMPORT-015.4 – Failed files still finish the job
- **Given** an import job for a source with two files, one of which cannot be read
- **When** it runs
- **Then** the summary reports one file copied and one failed, and the job ends as
  `Finished`

### IMPORT-015.5 – Stopping an import job
- **Given** an import job for a source with three files that is copying the first
- **When** it is asked to stop
- **Then** the first file is copied and verified, the other two are not started, the
  summary says the run was stopped, the import history records the end of the run, and
  the job ends as `Stopped`

### IMPORT-015.6 – A removed import job does not touch the source
- **Given** an import job queued behind another job
- **When** it is removed before it runs
- **Then** its source is not scanned, its enqueuer receives no report, and the job is
  reported `Removed`

### IMPORT-015.7 – An opened camera comes back
- **Given** an import job from an opened camera
- **When** it ends, as `Finished`, `Stopped` or `Failed`
- **Then** its end report hands the opened camera back to the enqueuer

### IMPORT-015.8 – Starting from a plan
- **Given** a planned run made from the import job's settings on the caller's thread
- **When** the import job is given that plan and runs
- **Then** it executes the plan without reporting a scan, copies its files, and records
  the run in the import history

### IMPORT-015.9 – Asking before each file
- **Given** an import job that asks its enqueuer before each file
- **When** the enqueuer answers that it should not go on
- **Then** no further file is started, the summary says the run was stopped, and the
  job is reported `Stopped`

### IMPORT-015.10 – A job waiting for an answer stops when asked
- **Given** an import job that asks its enqueuer before each file, and an enqueuer that
  holds its reports and does not follow them
- **When** the job is asked to stop
- **Then** no file is started, the summary says the run was stopped, and the job ends
  as `Stopped`

### IMPORT-015.11 – Resources of an import job
- **Given** an import job from a source folder on file system 1 to a destination on file
  system 2, with a backup destination on file system 3 and a video destination whose file
  system cannot be told
- **When** it is enqueued
- **Then** it names the source on file system 1, the destinations on file systems 2 and
  3, and the path of the video destination, and nothing else

### IMPORT-015.12 – Import jobs on separate devices run in parallel
- **Given** a running import job that asks its enqueuer before each file and gets no
  answer yet, and a second import job from another source to another destination file
  system
- **When** the second job is enqueued
- **Then** it runs to `Finished` and copies its files while the first still waits

### IMPORT-015.13 – Import jobs on one destination file system run one after another
- **Given** a running import job as in IMPORT-015.12, and a second import job from
  another source to a destination on the same file system
- **When** the second job is enqueued
- **Then** it is reported `Queued`, and `Running` only after the first ended

### IMPORT-015.14 – A reserved target name counts as taken
- **Given** a running import job that reserved the target `sub/a.jpg` in the
  destination `<library>` and waits before its first file, and a second import job into
  `<library>/sub`, on a file system that cannot be told, whose plan names `a.jpg` with
  other content, under `add_suffix`
- **When** the second job runs
- **Then** it copies the file to `a-1.jpg`

### IMPORT-015.15 – A job that cannot reserve is not enqueued
- **Given** a planned import job with `{seq_stored:2}.{ext}` whose numbers do not fit,
  because a job enqueued before it reserved them
- **When** it is enqueued
- **Then** the enqueuer gets the sequence error, the queue reports no state for it, and
  nothing is reserved

## Out of scope

- Planning and culling the next source while an import runs: J2.
- Queueing a newly inserted card in the guided import while a job runs: a later step
  of J2.
- Resuming an import job after a restart: J6; until then an import is resumed by
  running it again (IMPORT-009).

## History

| Date | Change |
|---|---|
| 2026-11-07 | Created (J1, ADR 0035), with starting from a plan and asking before each file |
| 2026-11-07 | Import jobs reserve at enqueue time, name their resources and run in parallel on separate devices; a job waiting for an answer stops when asked (J3, Q5, ADR 0036); criteria 10–15 |
| 2026-11-07 | Implemented: `ImportJob::enqueue`, `ReservedTargetNames` and the job's resources |
