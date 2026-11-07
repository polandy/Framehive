# IMPORT-015 – An import as a job

- Status: implemented
- Module: import
- Origin: J1
- Depends on: CORE-007, IMPORT-004, IMPORT-009, IMPORT-014

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
stops is stopped as above and ends as `Stopped`.

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

## Out of scope

- Planning and culling the next source while an import runs: J2.
- Two imports at the same time, and sequence numbers reserved at enqueue time: J3.
- Resuming an import job after a restart: J6; until then an import is resumed by
  running it again (IMPORT-009).

## History

| Date | Change |
|---|---|
| 2026-11-07 | Created (J1, ADR 0035), with starting from a plan and asking before each file |
