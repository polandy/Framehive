# 0029 – Persisted sequence counters

- Status: accepted
- Date: 2026-11-04

## Context

IMPORT-002 numbers imported files. Two kinds of counter outlive one run: `imported_today`
counts the imports of one calendar day and `stored` counts all imports ever, both per
machine. Their values must survive restarts, and two imports that run at the same time,
in two processes, must never get the same number. Losing a value is not harmless: a
counter that starts again at 1 produces names that already exist, and the user finds
suffixed copies (IMPORT-003) instead of a continuous numbering.

ADR 0025 keeps the answers of the guided import in a state file. That file may be lost
in a crash between removing the old file and renaming the new one, and two guided
imports write it last-writer-wins. Both are acceptable for remembered answers and not
for counters.

There is no job queue yet (J1). An import is either `framehive import` or one card of the
guided import, and each runs in its own process.

## Decision

### 1. A state file next to the guided import's

The counters live in `sequence-counters.txt` in the state folder of ADR 0025, section 1
(`$XDG_STATE_HOME/framehive/`, or `$HOME/.local/state/framehive/`). They are state, not
configuration: Framehive writes them, they have no JSON Schema, and they are not shared
between machines through the library (ADR 0003), so two machines never merge counters.

### 2. Plain text, the format of ADR 0025

```
# Framehive keeps its sequence counters here (IMPORT-002).
format_version=1
imported_today_date=2026-10-17
imported_today_last=15
stored_last=250
```

- The rules of ADR 0025, section 2, apply: `format_version=1` first, `<key>=<value>`
  lines, `#` comments and empty lines ignored.
- Each key appears at most once. A `*_last` value is the last number handed out, a
  decimal number from 0 to 999999; 0 means none yet. `imported_today_date` is the
  machine-local date (`YYYY-MM-DD`) that `imported_today_last` belongs to.
- A missing key means 0, and a missing `imported_today_date` means no import yet.

### 3. A missing file starts at 0, a broken file stops the import

A missing file is the first run: every counter is 0. A file that cannot be read, has
another `format_version`, a line Framehive does not understand, a key twice or a value
out of range stops the reservation with an error that names the file and the line. The
import does not start, and Framehive never overwrites the file. Unlike ADR 0025, a broken
file is not ignored, because starting again at 0 would produce names that already exist.
The user repairs or deletes the file.

### 4. Reserved under a lock, written by a replacing rename

A reservation is one read-modify-write of the file:

1. Open or create `sequence-counters.lock` in the same folder and take an exclusive lock
   on it with `std::fs::File::lock`. The lock waits for the other process, which holds
   it only for the few milliseconds of its own reservation. The lock file is never
   renamed or removed, so every process locks the same file.
2. Read `sequence-counters.txt`, compute the new values (IMPORT-002), and write them to
   `sequence-counters.txt.part`.
3. Sync the `.part` file, rename it over `sequence-counters.txt`, and sync the folder.
   The rename replaces the old file in one step on Linux and macOS, so a crash leaves
   either the old or the new file, never none.
4. Unlock by closing the lock file. The operating system also releases the lock when a
   process dies.

A reservation hands out a contiguous range and is written before the first file is
copied. When the import is cancelled or fails later, its numbers stay used. Gaps are
allowed; duplicates are not.

### 5. Where the code lives

The service `naming` owns the counters' rules as pure functions: the day reset, the
contiguous range, the overflow check. It defines a port `SequenceCounters` whose one
operation runs such a function on the stored values under the lock and stores the
result. An adapter crate implements the port with the file, the lock and the rename of
section 4; it uses `std::fs` directly, because the `FileSystem` port has no replacing
rename and no lock, and the counters need both. The import module calls the port when an
import starts, not when it is planned: a dry run and the plan summary of the guided
import show the numbers the import would get, and reserve nothing.

## Alternatives

- **Remove, then rename, as in ADR 0025:** a crash between the two steps loses the
  counters, see the context.
- **Lock the counter file itself:** the replacing rename puts a new file in its place, and
  a process waiting on the old file would read stale values.
- **One file per counter, or a SQLite database:** more files or a native dependency
  (ADR 0009) for three values. The import history (N3) may bring a database later; the
  counters can move into it with a superseding ADR.
- **Reserve when the plan is made:** every dry run and every plan summary would use up
  numbers.

## Consequences

- Deleting the file restarts every counter at 1; Framehive does not prevent it.
- `std::fs::File::lock` needs Rust 1.89 or later; the pinned toolchain is newer.
- Files already imported are recognized by their target path (IMPORT-003). With a
  persisted counter, the same card imported again gets new numbers and is copied again.
  IMPORT-002 makes the import plan say so, until the import history (N3) recognizes
  files by their content.
