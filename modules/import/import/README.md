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
asset of only sidecars takes its base name from its first file.

A job code (IMPORT-006) is validated by `JobCode::parse`: trimmed, 1–64 characters,
no `/`, no control character and not dots only. The plan passes it to the `{job}` token, and
`require_job_code` refuses an import whose template uses `{job}` without one.
`RecentJobCodes` keeps the job codes used before, the most recently used first, each
once and at most ten; the caller stores and restores it.

The source scan (IMPORT-005) walks the source folder and lists each supported file
with its role, size and modification time, and each file or folder it leaves out with
the reason: unsupported type, hidden, symbolic link, sidecar until grouping, or cannot
be read. `TargetResolver` resolves name conflicts and files already imported
(IMPORT-003) against the destination and the earlier files of the plan, comparing
names by their canonical caseless key, without regard to case and Unicode
normalization form (ADR 0026); a destination may hold several names with one key,
and each is compared. Content is compared by size and checksum, under the conflict
policy `add_suffix`, `skip` or `fail`.

`ImportRun` ties them together (ADR 0020 §1): it scans the source, dates each file by
its capture time or, without one, by its modification time in local time, makes one
import plan, resolves each target and copies the file with `files::copy_verified`
(IMPORT-004), in scan order. Until grouping exists, each file is its own asset. A file
that fails is reported with the reason and the run goes on (IMPORT-004.11). A dry run
resolves everything and writes nothing.

Backup destinations (IMPORT-007, ADR 0030): `ImportSettings::backup_destinations`
lists them. `start` (and so `run`) refuses one that is the import destination, lies
inside it or the source, contains it, or is given twice (`check_backup_destinations`).
A file copied to the import destination, or already imported there, goes to the same
target path below each backup destination, with one read of the source through
`files::copy_verified_to_all`. A backup target holding the same content is "already
backed up"; one holding other content is a name conflict there and is never
overwritten. Each destination fails on its own: a failed backup does not undo the
import, and a failed import destination keeps the verified backups.
`FileReport::backups` holds one `BackupReport` per backup destination, and
`ImportSummary::backups` the `BackupCounts` of each.

Format readiness (IMPORT-008): `FormatReadiness::of` reads an `ImportSummary` and tells
whether the source can be formatted: every file the scan found is verified at the
import destination and at every backup destination (a duplicate counts as its earlier
file), nothing was left out as unreadable or as a sidecar that needs grouping, and the
run was neither stopped nor a dry run (`ImportSummary::dry_run`). Otherwise
`FormatReadiness::blockers` lists each file that is not saved, with the number of
locations it is verified at, and each blocking left-out entry, in scan order.
Unsupported, hidden and symbolic-link entries are only counted.

`ImportRun::run` is the composition of two steps that callers can also use on their
own. `plan` (all files of a scan) or `plan_first` (the first files only, for a preview)
fixes the import time, dates only the selected files and returns a `PlannedRun`: each
file with its planned target, or why it could not be dated. It resolves no conflicts
and writes nothing. `execute` takes a `PlannedRun`, resolves conflicts and copies, with
the same progress steps and summary as `run`.

Sequence numbers (IMPORT-002): the plan numbers assets, not files, in shooting order
(capture time, then base name byte by byte), from one contiguous range. `plan` only
previews the range; `start` reserves it through the `naming::SequenceCounters` port, or
from the `SessionSequence` for session and letter numbers, before the first file is
copied, and names the files again if another import took the previewed numbers. A dry
run reserves nothing. `run` is the scan, `plan`, `start` and `execute`.

Import history (IMPORT-009): `ImportRun` reads and writes the `history::ImportHistory`
port. `plan` looks each file up by original file name, size and capture time (before
camera time correction) at the import destination and at each backup destination, and
reads its checksum only when a lookup finds a copy. A file whose most recent copy at the
import destination has the same checksum is `FilePlan::ImportedBefore`: it is left out of
the import plan, gets no sequence number, and is reported as `FileOutcome::ImportedBefore`
with the recorded target, without looking at the destination. It is still backed up to
each backup destination where it is not `BackupOutcome::BackedUpBefore`. `start` records
the run, `execute` records each verified copy (copied, already imported, backed up,
already backed up) right after it is verified and the end of the run, also when it is
stopped. A dry run records nothing. A history that cannot be read stops the plan
(`RunError::History`); one that cannot record stops the run after that file
(`ImportSummary::history_error`), which format readiness treats like a stopped run.
The run compares destinations exactly as `ImportSettings` gives them, so the caller
passes their canonical absolute paths (IMPORT-009); `framehive-cli` does (CLI-008).

Boundaries:

- **Pure logic, I/O through ports.** Making an import plan does no I/O. The scan,
  conflict resolution and the import run read and write only through the `FileSystem`
  and `Hasher` ports of `files`, the `Clock` of `domain`, the `SequenceCounters` of
  `naming` and the `ImportHistory` of `history`; the app wires the
  adapters. `ImportRun` runs synchronously on the caller's thread and reports to a
  `ProgressSink`, which it asks before each file whether to go on
  (`should_continue`, IMPORT-004.13); a stopped run sets `ImportSummary::stopped`. It
  becomes the body of the import job once the jobs service exists.
- **Never writes to the source** and never replaces a file at the destination
  (ADR 0020 §2).
- **Grouping** of files into assets is defined by LIBRARY-001; the module uses it and
  does not redefine it. `ImportPlan::new` takes assets that are already grouped.
- **Not covered yet:** metadata tokens,
  camera time correction, sidecars (until grouping), the import log, the live preview and block editor (a UI spec), and
  where a profile stores its template (the profile spec).

## Public API

| Item | Meaning |
|---|---|
| `MODULE_ID`, `module_id` | Stable id of the module (architecture §4.2) |
| `ImportPlan` | Pure description of which file goes where, with what name (IMPORT-001) |
| `AssetFile` | One file of an asset as the import finds it: name, role, capture and modification time |
| `PlannedFile` | The target path of one file, or why it cannot be named, its date and its date source |
| `scan_source`, `scan_folder`, `SourceScan`, `FoundFile`, `LeftOut`, `LeftOutReason`, `ScanError` | The source scan and its result (IMPORT-005); `scan_folder` scans without the destination check |
| `TargetResolver`, `TargetRequest`, `ConflictPolicy`, `ConflictOutcome`, `ConflictError` | Conflict resolution against the destination and the plan (IMPORT-003) |
| `JobCode`, `JobCodeError`, `MAX_JOB_CODE_CHARACTERS` | A validated job code and why a text is not one (IMPORT-006) |
| `RecentJobCodes`, `MAX_RECENT_JOB_CODES` | The job codes used before, most recent first (IMPORT-006) |
| `require_job_code`, `MissingJobCode` | Refuses an import whose template uses `{job}` without a job code (IMPORT-006) |
| `ImportRun`, `ImportSettings` | Scans, plans, starts, resolves and copies, file by file (ADR 0020, IMPORT-004) |
| `SessionSequence` | The session counter of one run of the application (IMPORT-002) |
| `PlannedRun`, `RunFile`, `FilePlan` | An import run planned and not yet executed: each selected file with its plan entry, imported before, or its dating failure |
| `ProgressSink`, `NoProgress` | Receives each step of an import run on the caller's thread |
| `ImportSummary`, `FileReport`, `FileOutcome`, `FileError`, `RunError` | What a run reports per file, and why a run did not start |
| `BackupReport`, `BackupOutcome`, `BackupError`, `BackupCounts` | What happened to a file at each backup destination, and the counts per backup destination (IMPORT-007) |
| `check_backup_destinations`, `BackupDestinationError`, `BackupDestinationProblem` | Refuses invalid backup destinations before a run starts (IMPORT-007.11) |
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

- `domain`
- `files`
- `kernel`
- `metadata`
- `naming`
