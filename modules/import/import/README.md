<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# import

Layer: `module` (architecture §3, §4)

## Purpose

The import module brings files from a source (camera, card, phone, folder) into the
library. It decides where each file goes and what it is called, and describes that
decision as an import plan before anything is copied.

Its first feature is the name template (IMPORT-001): `ImportPlan` renders a name
template from the naming service for each asset file, giving its folders and name
relative to the import destination. All files of one asset get the same values: the
capture time chosen by `naming::asset_date`, and the base name of the asset's first
file that is not a sidecar, so sidecars follow their file. A file that cannot be named
is marked with the reason, and a file whose asset has no capture time is marked as
dated by its modification time. An asset without files adds nothing to the plan; an
asset of only sidecars takes its base name from its first file. With a video name
template (IMPORT-018), a file whose role is `Video` is named by it instead; both
templates share the job code and one numbering, and a run whose templates number by
different sequence kinds does not start (`MixedSequenceKinds`).

A job code (IMPORT-006) is validated by `JobCode::parse`: trimmed, 1–64 characters,
no `/`, no control character and not dots only. The plan passes it to the `{job}` token, and
`require_job_code` and `require_video_job_code` refuse an import whose name template or
video name template uses `{job}` without one.
`RecentJobCodes` keeps the job codes used before, the most recently used first, each
once and at most ten; the caller stores and restores it.

The source scan (IMPORT-005, IMPORT-014) and conflict resolution (IMPORT-003,
IMPORT-019) are described in [`docs/scan-and-conflicts.md`](docs/scan-and-conflicts.md).

`ImportRun` ties them together (ADR 0020 §1): it scans, plans, starts and copies,
with backup destinations, format readiness, sequence numbers, the import history and
the free-space check, and writes each photo's XMP sidecar (IMPORT-019); a run that
would write sidecars without `with_xml_syntax` does not start
(`RunError::SidecarsWithoutXmlSyntax`). [`docs/import-run.md`](docs/import-run.md)
describes it.

Boundaries:

- **Pure logic, I/O through ports.** Making an import plan does no I/O. The scan,
  conflict resolution and the import run read and write only through the
  `FileSystem`, `Hasher` and `FreeSpace` ports of `files`, the `Clock` of `domain`,
  the `SequenceCounters` of `naming` and the `ImportHistory` of `history`; the app
  wires the adapters. `ImportRun` runs synchronously on the caller's thread and reports to a
  `ProgressSink`, which it asks before each file whether to go on
  (`should_continue`, IMPORT-004.13); a stopped run sets `ImportSummary::stopped`. It
  is the body of the import job (IMPORT-015), which runs it on a worker thread of the
  job queue and sends each step to its enqueuer.
- **Never writes to the source** and never replaces a file at the destination
  (ADR 0020 §2). The scan, dating, recognition, conflict resolution and the copy read
  the source only through the `ReadOnlyFileSystem` port (ADR 0034). The source lies on
  the file system of the destinations unless `ImportRun::with_source` gives it one of
  its own, such as an opened camera.
- **Grouping** of files into assets is defined by LIBRARY-001; the module uses it and
  does not redefine it. `ImportPlan::new` takes assets that are already grouped.
- **Not covered yet:** metadata tokens, camera time correction, sidecars (until
  grouping), the import log, and the live preview and block editor (a UI spec).

## Public API

| Item | Meaning |
|---|---|
| `MODULE_ID`, `module_id` | Stable id of the module (architecture §4.2) |
| `ImportPlan` | Pure description of which file goes where, with what name (IMPORT-001) |
| `AssetFile` | One file of an asset as the import finds it: name, role, capture and modification time |
| `PlannedFile` | The target path of one file, or why it cannot be named, its date and its date source |
| `scan_camera` | The scan of an opened camera: `DCIM` of each storage only (IMPORT-014) |
| `FormatReadiness`, `FormatBlocker`, `CAMERA_FORMAT_READINESS_NOTE` | Whether the source can be formatted and what keeps it (IMPORT-008); the note of a camera run (IMPORT-014.10) |
| `scan_source`, `scan_folder`, `SourceScan`, `FoundFile`, `LeftOut`, `LeftOutReason`, `ScanError` | The source scan and its result (IMPORT-005); `scan_folder` scans without the destination check |
| `TargetResolver`, `TargetRequest`, `ConflictPolicy`, `ConflictOutcome`, `ConflictError` | Conflict resolution against the destination and the plan (IMPORT-003) |
| `JobCode`, `JobCodeError`, `MAX_JOB_CODE_CHARACTERS` | A validated job code and why a text is not one (IMPORT-006) |
| `RecentJobCodes`, `MAX_RECENT_JOB_CODES` | The job codes used before, most recent first (IMPORT-006) |
| `require_job_code`, `require_video_job_code`, `MissingJobCode` | Refuses an import whose name template or video name template uses `{job}` without a job code (IMPORT-006, IMPORT-018) |
| `MixedSequenceKinds` | The name template and the video name template number by different sequence kinds (IMPORT-018) |
| `ImportRun`, `ImportSettings` | Scans, plans, starts, resolves and copies, file by file (ADR 0020, IMPORT-004); `with_source` reads the source from its own `ReadOnlyFileSystem`, against which no destination is checked, and `with_source_name` names it in the import history, such as `camera:<model>` (ADR 0034) |
| `ImportJob`, `ImportJobReports`, `ImportJobEnd`, `ImportPorts`, `ImportJobSettings`, `OwnSource` | An import run as a job of the `jobs` queue (IMPORT-015, ADR 0035): it owns its ports, settings and source; `with_plan` starts from a planned run, `asking_before_each_file` asks the enqueuer whether to go on, and a stop request answers "no"; `enqueue` reserves a planned job's sequence range and target names and enqueues it; the job names its source device and destination file systems as resources (ADR 0036); `ImportJobReports::follow` shows its reports to a `ProgressSink` and returns the end report with the summary and the source handed back |
| `ReservedTargetNames` | The target names the import jobs of one application reserved; another job's name counts as taken (IMPORT-015, ADR 0036 §6) |
| `DryRunMemory` | The counters and target names the dry-run jobs of one command reserve in memory, over the persisted counters, which it never writes; given with `ImportJob::with_dry_run_memory`, a dry-run job reserves at enqueue and keeps its names after it ends (IMPORT-016) |
| `SessionSequence` | The session counter of one run of the application (IMPORT-002) |
| `PlannedRun`, `RunFile`, `FilePlan` | An import run planned and not yet executed: each selected file with its plan entry, imported before, or its dating failure |
| `ProgressSink`, `NoProgress` | Receives each step of an import run on the caller's thread |
| `ImportSummary`, `FileReport`, `FileOutcome`, `FileError`, `RunError`, `DestinationNotCreated` | What a run reports per file, and why a run did not start, such as a destination it could not create when it started (IMPORT-012); each `FileReport` carries its own required locations and whether it went to a separate video destination (IMPORT-013) |
| `BackupReport`, `BackupOutcome`, `BackupError`, `BackupCounts` | What happened to a file at each backup destination, and the counts per backup destination (IMPORT-007) |
| `check_backup_destinations`, `BackupDestinationError`, `BackupDestinationProblem` | Refuses invalid backup destinations before a run starts (IMPORT-007.11) |
| `check_video_destination`, `VideoDestinationError`, `VideoDestinationProblem`, `check_against_video_destination` | Refuses an invalid video destination, and backup destinations that clash with it, before a run starts (IMPORT-013.8) |
| `FreeSpaceCheck`, `SpaceShortage`, `FreeSpaceUnknown`, `NotEnoughSpace` | What the free-space check before the first copy finds (IMPORT-012) |
| `ImportProfile`, `IMPORT_PROFILE`, `IMPORT_PROFILE_V1`, `DEFAULT_NAME_TEMPLATE`, `import_profile_json_schema`, `import_profile_v1_json_schema`, `IMPORT_PROFILE_SCHEMA_ID`, `IMPORT_PROFILE_V1_SCHEMA_ID` | An import profile read through `config`, checked, with defaults; its JSON Schemas, versions 2 and 1 (IMPORT-011; tested end to end in `apps/cli`) |
| `Configuration`, `CONFIGURATION`, `CONFIGURATION_FILE_NAME`, `configuration_json_schema`, `CONFIGURATION_SCHEMA_ID` | `config.yaml` and its JSON Schema (IMPORT-019) |
| `MetadataKeys`, `ImportMetadata`, `Keyword`, `KeywordError` | The `metadata` keys, layered, and the values the import writes (IMPORT-019) |
| `ExampleViewModel` | Example ViewModel from the template; replace it |

## Events

Published: none yet. Subscribed: none yet. New events need an ADR (architecture §4.5).

## Commands

Handled: none yet. Sent: none yet. New commands need an ADR (architecture §4.5).

## Data

No tables yet. Tables use the prefix `import_` and describe their rebuild
(architecture §4.4).

## Extension points

None yet.

## Dependencies

- `config`
- `domain`
- `files`
- `jobs`
- `kernel`
- `metadata`
- `naming`
