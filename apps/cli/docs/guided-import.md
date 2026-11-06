<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# framehive-cli: guided import

`framehive` without a subcommand runs the dialog of CLI-002 and CLI-003 in
`src/guided/`: it waits for a memory card (`devices::DeviceWatch` over `adapter-mounts`, polled once a second
while it waits for a line), reads the card with `import::scan_folder`, asks for the
template (with a preview from `ImportRun::plan_first`) and the job code, shows the plan
summary, reserves the sequence numbers with `ImportRun::start` on Enter (numbers that
do not fit lead back to the template step), and runs `ImportRun::execute` with a progress sink that redraws one line and
stops the run between files when the card is pulled (IMPORT-004.13). The dialog is a
loop over input events (a line, a tick per poll interval, the end of input) from a
thread that reads standard input. Backup destinations (CLI-006) are chosen with `b` at
the plan summary and passed to the run; one on the card, or one that
`import::check_backup_destinations` rejects, blocks Enter like a destination on the
card, and the report adds the files with a failed backup and the backup summary lines.
The import history is `adapter-sqlite` in the state folder; the destinations are resolved
with `paths::canonical_folder` (through `Ports::resolve_folder`) before each plan, and a
history that cannot be used is reported as `The import history <path> cannot be used:
<reason>`.
The destination, the backup destinations, the template and the recent job codes are
remembered in `$XDG_STATE_HOME/framehive/guided-import.txt` (ADR 0025, format version 2
of ADR 0030; version 1 is still read), written through the `FileSystem` port as
`<file>.part`, then renamed. It exits with 0
when the person quits, and with 2 when it cannot start: standard input or output is not
a terminal, or the home folder or the current folder is unknown.

## Tests

The guided import is tested in the crate (`src/guided/tests/`) with golden transcripts:
the dialog runs against scripted input steps, `MemoryFileSystem`, `MemoryDeviceSource`
and a `ManualClock`; `tests/guided_import.rs` checks the binary without a terminal.
