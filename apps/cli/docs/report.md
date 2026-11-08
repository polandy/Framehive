<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# framehive-cli: report and exit codes

The report texts live here, not in the import module. Standard output gets one line
per file as soon as it is done (`copied`, `would copy`, `already imported`,
`imported before`, `duplicate`, `skipped`, `failed`), each followed by one indented line
per backup destination (`backed up`, `already backed up`, `backed up before`,
`backup failed`) that names the folder
as given, the sidecar lines of CLI-017 (`wrote sidecar`, `replaced`, `kept`) before the
backup lines, the files and folders the scan left out (`left out`), a summary line, and one
`backup <folder>: …` line per backup destination; in a dry run `copied` reads
`would copy`, in the summary as well, `wrote sidecar` reads `would write sidecar`, and
there are no backup lines. A run that wrote sidecars adds `<n> sidecars written` after
the summary line (`sidecars_written_line`). The format
readiness (CLI-007, `format_readiness_lines`) ends the report of a run that is not a
dry run: `framehive import` says `source`, the guided import says `card` and prints it
before the eject question. Errors that stop the command go to standard error as
`error: <message>`, and so does an import history that fails during the run, after the
summary. With several sources (CLI-015), each source's summary block follows a line
`<source as given>:`, and each error that kept a source from starting reads
`error: <source as given>: <message>` at once (`Streams::reported`); the exit code is
the highest of the sources'.
A job code that the template does not use is accepted and reported on
standard error as `warning: <message>`; there is no other log output yet.

| Code | Meaning |
|---|---|
| 0 | Every file was copied, already imported, imported before, a duplicate, skipped or left out |
| 1 | A file failed, or a folder could not be read (`ImportSummary::has_failures`), a file failed at a backup destination, or the import history failed during the run |
| 2 | The import did not start: invalid arguments, template or job code, missing source, destination inside the source or not creatable, an invalid or not creatable backup destination, sequence numbers that do not fit, counters that cannot be read or stored, an unknown state folder, an import history that cannot be used, a file system without enough space, or a profile that cannot be found, read or used (CLI-010) |
