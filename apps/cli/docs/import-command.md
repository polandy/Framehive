<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# framehive-cli: the import command

`framehive import` parses the name template, checks `--job-code` with
`import::JobCode` and `import::require_job_code` (IMPORT-006), resolves the source without symbolic
links, resolves the longest existing part of the destination and appends the missing
folders (rejecting `..` among them), and rejects a destination inside the source.
Each `--backup` folder (CLI-005) is resolved the same way and checked with
`import::check_backup_destinations` (IMPORT-007.11), also in a dry run.
So are `--video-destination` and `--video-backup` (CLI-012, IMPORT-013.8). These checks
run for every source before any job (`folders::resolve_sources`), which also rejects a
source given twice or lying inside another (CLI-016). It scans and
plans, and checks the free space with `adapter-free-space` (CLI-011): a short file system
stops it with exit code 2 before any folder is created, a dry run after its lines.
Unless it is a dry run, it then creates the missing folders of all destinations, and
starts and executes the run with
`adapter-fs`, `adapter-blake3` and `adapter-clock`, in compatibility mode. The persisted sequence counters (CLI-004, ADR 0029) come from
`adapter-sequence-counters`, and the import history (CLI-008, ADR 0031) from
`adapter-sqlite`, both in the state folder; the command stops when the state folder is
unknown (no absolute `XDG_STATE_HOME` and no home folder). The run gets the resolved paths,
by which the history identifies a destination (IMPORT-009).

## Several sources

The last positional argument is the destination, unless `--destination` names it; the
others are the sources, as `cp` has them (CLI-015, CLI-016). `locations()`
(`src/import_command/locations.rs`) splits them from the arguments alone, before the
profile is read and before any folder is looked at, and stops when there is no source,
no destination, or a camera where the destination belongs. A profile never names the
destination (CLI-010, IMPORT-011). Each source becomes an `import::ImportJob` of its own in one
`jobs::JobQueue`, and all of them share one `ImportPorts`: the counters, the session
and the `ReservedTargetNames`. The settings are chosen once; then each source is
checked, scanned, planned and enqueued in the order given, so `ImportJob::enqueue`
reserves its numbers and target names after those of the source before it (IMPORT-015).
In a dry run, the ports' counters are an `import::DryRunMemory` over the persisted
ones, and each job gets it with `with_dry_run_memory`, so the jobs reserve in memory
and the preview matches a real run (IMPORT-016).
The queue runs the jobs whose sources and destinations share no device in parallel.
A thread per job (`std::thread::scope`, `src/import_command/sources.rs`) follows its
reports with a `ReportPrinter` that writes to a buffer, and passes each report to
standard output in one piece, so parallel jobs interleave only whole lines. A source
that cannot start does not keep the others from importing.
