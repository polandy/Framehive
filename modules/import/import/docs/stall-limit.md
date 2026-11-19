<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# The stall limit of the source

An `ImportJob` reads its source through a `files::StallGuard` with
`ImportJobSettings::source_stall_limit` (IMPORT-022, ADR 0049), 30 seconds
(`DEFAULT_SOURCE_STALL_LIMIT`) unless the profile sets `source_stall_limit_seconds`.
`ImportPorts::guard_source` builds a `SourceGuard` from the job's own source, or else
from the shared file system of the ports, and runs its calls on a thread that the
`ImportPorts::timer` limits. The thread only borrows the source: dropping the
`SourceGuard`, or `SourceGuard::give_back`, takes it back at once, so a camera is
released when its job ends and not whenever the thread ends. For the shared file system the run uses
`ImportRun::reading_source_through`, which reads the source through the guard and
still checks the destinations against the source; an own source keeps
`with_source`. The job hands back its own source with `give_back`: the source itself, or the guard
when a call did not answer, as the source then stays with that call.

A read past the limit fails that file with the stall of the guard, and every later
call fails at once, so the remaining files of the source fail without a call to it. A
listing past the limit fails the scan with `ScanError::SourceStalled`, which names the
folder. A guard whose thread cannot start ends the job with `RunError::SourceThread`.

A caller that scans and plans before the job, such as `framehive import`, makes its
own guard with `guard_source`, so a hang there fails as well; the job then guards the
copy with a guard of its own.
