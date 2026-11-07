<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# The import run

How `ImportRun` of the `import` crate scans, plans, starts and copies. The crate's
[README](../README.md) has its purpose, public API and boundaries.

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

Free space (IMPORT-012, ADR 0033): `start` checks, after the backup destinations and
before it reserves numbers or copies anything, that each file system can hold what the
run writes to all destinations on it; a dry run checks too. `check_free_space` resolves
each planned file as the run would and counts its size at the import destination if it
is copied, and at each backup destination whose target is free and where it is not
backed up before. Destinations are grouped by the `FileSystemId` the `FreeSpace` port
reports. A short file system stops the run with `RunError::NotEnoughSpace`, listing
every short one; a destination whose free space cannot be read is kept in
`ImportSummary::free_space_unknown`, and the run goes on.

Creating the destinations (IMPORT-012): only once these checks pass, and not in a dry
run, `start` creates every missing destination of `ImportSettings::all_destinations`
with its parents through the `FileSystem` port, and syncs each new folder into its
parent. A folder that already exists, also one created meanwhile, is used as it is.
Callers therefore never create the destinations themselves: a run that cannot start,
also one whose job waited in the job queue while another job filled the disk, leaves
no folder behind. A folder that cannot be created stops the run with
`RunError::DestinationNotCreated` before anything is reserved or recorded; it names
the destination, the folder that failed and the reason.
