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

The source scan (IMPORT-005) walks the source folder and lists each supported file
with its role, size and modification time, and each file or folder it leaves out with
the reason: unsupported type, hidden, symbolic link, sidecar until grouping, or cannot
be read. `TargetResolver` resolves name conflicts and files already imported
(IMPORT-003) against the destination and the earlier files of the plan, comparing
names without regard to case and content by size and checksum, under the conflict
policy `add_suffix`, `skip` or `fail`.

`ImportRun` ties them together (ADR 0020 §1): it scans the source, dates each file by
its capture time or, without one, by its modification time in local time, makes one
import plan, resolves each target and copies the file with `files::copy_verified`
(IMPORT-004), in scan order. Until grouping exists, each file is its own asset. A file
that fails is reported with the reason and the run goes on (IMPORT-004.11). A dry run
resolves everything and writes nothing.

Boundaries:

- **Pure logic, I/O through ports.** Making an import plan does no I/O. The scan,
  conflict resolution and the import run read and write only through the `FileSystem`
  and `Hasher` ports of `files` and the `Clock` of `domain`; the app wires the
  adapters. `ImportRun` runs synchronously on the caller's thread and reports to a
  `ProgressSink`; it becomes the body of the import job once the jobs service exists.
- **Never writes to the source** and never replaces a file at the destination
  (ADR 0020 §2).
- **Grouping** of files into assets is defined by LIBRARY-001; the module uses it and
  does not redefine it. `ImportPlan::new` takes assets that are already grouped.
- **Not covered yet:** sequence numbers (IMPORT-002), metadata tokens, the remembered
  list of job codes,
  camera time correction, sidecars (until grouping), backup destinations, resuming an
  interrupted import, the import log, the live preview and block editor (a UI spec), and
  where a profile stores its template (the profile spec).

## Public API

| Item | Meaning |
|---|---|
| `MODULE_ID`, `module_id` | Stable id of the module (architecture §4.2) |
| `ImportPlan` | Pure description of which file goes where, with what name (IMPORT-001) |
| `AssetFile` | One file of an asset as the import finds it: name, role, capture and modification time |
| `PlannedFile` | The target path of one file, or why it cannot be named, and its date source |
| `scan_source`, `SourceScan`, `FoundFile`, `LeftOut`, `LeftOutReason`, `ScanError` | The source scan and its result (IMPORT-005) |
| `TargetResolver`, `TargetRequest`, `ConflictPolicy`, `ConflictOutcome`, `ConflictError` | Conflict resolution against the destination and the plan (IMPORT-003) |
| `JobCode`, `JobCodeError`, `MAX_JOB_CODE_CHARACTERS` | A validated job code and why a text is not one (IMPORT-006) |
| `require_job_code`, `MissingJobCode` | Refuses an import whose template uses `{job}` without a job code (IMPORT-006) |
| `ImportRun`, `ImportSettings` | Scans, plans, resolves and copies, file by file (ADR 0020, IMPORT-004) |
| `ProgressSink`, `NoProgress` | Receives each step of an import run on the caller's thread |
| `ImportSummary`, `FileReport`, `FileOutcome`, `FileError`, `RunError` | What a run reports per file, and why a run did not start |
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
