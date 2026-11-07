<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# framehive-cli: import log

`framehive import-log` (CLI-009) opens the import history of the state folder with
`SqliteImportHistory::read_only`, so it never creates the history or the state folder.
Without `<run>` it prints one line per run from `ImportHistory::list_runs`; with a run
number or `last` it prints `ImportHistory::read_run_log` as text, JSON or RFC 4180 CSV
(`src/import_log/`, own writers, ADR 0009). Paths are printed
lossy (U+FFFD for invalid UTF-8); the text format also escapes control characters,
so a name with a line break stays on one line. `--format` without `<run>` is a
usage error. A run that does not exist, `last` without runs, a history that cannot be
used and an unknown state folder print `error: …` and exit with 2.
