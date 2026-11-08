# 0031 – Import history in SQLite

- Status: accepted
- Date: 2026-11-04

## Context

The import recognizes a file already imported only by its target path (IMPORT-003).
That fails in three cases the requirements name: a persisted sequence counter gives the
same file a new name on every import (IMPORT-002), so a card imported again is copied
again; a file renamed or moved in the library is no longer found at its target path
(N3, T8); and an interrupted import with a persisted counter copies the files it had
already verified once more when it is started again (N4, T1).

The import history (N3) closes these gaps. It records every verified copy of an import
run, so a later run recognizes the file by its content, wherever the library keeps it
now. It is also the data of the import log (N6) and of the pairing of RAW and JPEG files
across separate import jobs (LIBRARY-001).

The history grows with every import, to tens of thousands of rows within a few years.
Each file of a run needs an indexed lookup, and two imports may run at the same time in
two processes (ADR 0029, context). The architecture already plans `adapter-sqlite` and a
machine-local SQLite index (architecture §2, ADR 0003).

## Decision

### 1. SQLite through rusqlite, built from source

The history is a SQLite database, accessed through the crate `rusqlite` with its
`bundled` feature. `bundled` compiles the SQLite amalgamation that `rusqlite` ships with
the C compiler of the Nix shell, so every machine uses the same SQLite version and no
system library is needed. This is the first native library for data and is accepted
here (ADR 0009). `rusqlite` is used only in `adapter-sqlite`.

### 2. One database per machine, in the state folder

The database is `import-history.sqlite` in the state folder of ADR 0025, section 1
(`$XDG_STATE_HOME/framehive/`, or `$HOME/.local/state/framehive/`). It is state: Framehive
writes it, it has no JSON Schema, and it is not documented as something to edit. It never
lives in the library, because SQLite must not live on a network or sync drive (ADR 0003).
Two machines therefore do not know each other's imports; IMPORT-003 still recognizes a
file at an unchanged target path.

### 3. What a row records

A run is recorded when it starts and when it ends. Each verified copy is one row:

| Table | Columns |
|---|---|
| `runs` | `id`, `started_at`, `finished_at` (empty while running or after a crash), `source`, `import_destination` |
| `copies` | `run_id`, `destination`, `source_path`, `original_file_name`, `size`, `capture_time` (empty without one), `checksum`, `target`, `recorded_at` |

- `destination` is the import destination or one backup destination, as the canonical
  absolute path the run uses for its checks (IMPORT-007, "Before the run"). Folder
  paths are stored as their bytes (a BLOB), so a path that is not valid UTF-8 works too.
- `checksum` is written `blake3:<hex>` (architecture §8); `target` is the target path
  relative to the destination, with `/` between segments.
- `capture_time` is the capture time the file records, before camera time correction,
  as `YYYY-MM-DDTHH:MM:SS`. A later correction therefore does not hide a file.
- Times come from the `Clock` port, as local time with its UTC offset (RFC 3339).
- An index on `(destination, original_file_name, size)` serves the lookup of section 5.
- The schema version is SQLite's `user_version`, 1 for this layout.

### 4. Written per copy, in its own transaction

A copy is recorded right after it is verified, or found with identical content, at its
destination, in one transaction of its own. The database uses the WAL journal with
`synchronous=FULL`, so a recorded row survives a crash and a power loss, and a second
process reads while the first writes. A process that waits for another's write waits at
most 5 seconds (`busy_timeout`), then the import stops with an error. A dry run reads
the history and writes nothing.

### 5. Recognized by name and size first, then by content

Looking up a file is cheap first and exact last. The run looks for rows of the same
destination with the same original file name and size, and, when the file records a
capture time, the same capture time. Only when such a row exists, the source file is read
and its checksum compared with the row's checksum. A file is recognized only by an equal
checksum, so a match of name and size alone never skips a file. The lookup happens when
the plan is made, so a recognized file gets no sequence number.

### 6. A missing database starts empty, a broken one stops the import

A missing database is created with the schema of section 3. A database that cannot be
opened, is not a SQLite database, or has another `user_version` stops the import before
any file is copied, with an error that names the file. Framehive never deletes or
overwrites it; the user repairs or deletes it. Deleting it only forgets the history:
nothing in the library depends on it.

### 7. Where the code lives

A new service crate `history` (layer service) owns the history's types and the port
`ImportHistory`: record a run's start and end, record a copy, and find the rows that
match a name, a size and a capture time at a destination. It has a conformance suite in
`history::testing`. The import module calls the port; `adapter-sqlite` implements it with
section 3 to 6, and only `apps/cli` knows that adapter. The service is not part of the
import module because the library module needs the same history (LIBRARY-001), and a
module never depends on another module.

Addendum (2026-11-04, IMPORT-010): the import log reads the history back through two
more operations of the port: list the runs, oldest first, with the number of their
copies at every destination, and read the log of one run, its copies ordered by
destination as IMPORT-010 says, or nothing when there is no such run. For the import
log, `adapter-sqlite` opens the database with a read-only connection: a missing database
is an empty history and is not created, nor is its folder; a broken one or one with
another layout is the error of section 6. SQLite may leave its empty `-wal` and `-shm`
side files next to a database in WAL mode; they hold no data. The layout of section 3,
its `user_version` and the dependencies stay as they are.

## Alternatives

- **An append-only text file of our own:** no new dependency, but every lookup reads the
  whole file, two writers need a lock of their own, and a crash in the middle of a line
  needs its own recovery. SQLite solves all three and is tested far beyond what we could
  test.
- **The history in the library, under `.framehive/`:** every machine would see every
  import, but SQLite on a sync drive breaks (ADR 0003), and a text format brings back
  the first alternative with sync conflicts on top.
- **Hash every source file before planning:** recognizes files without name and size, but
  reads the whole card twice on every import. Cameras do not rename files on the card,
  so name and size are a safe first filter.
- **Record only the import destination:** an interrupted run that verified a file at the
  import destination but not at a backup destination would skip the backup when it is
  started again. Recording each destination lets the backup be completed.

## Consequences

- A native C library is compiled into the binary; its build needs the C compiler of the
  Nix shell, as LibRaw does (ADR 0028).
- A process killed after a file is verified but before its row is written copies that one
  file again on the next run, with a new sequence number when the template has a
  persisted counter. At most one file per destination is affected.
- Two imports of the same card into the same destination at the same time may both copy a
  file, because neither has recorded it yet.
- The sequence counters (ADR 0029) could move into the database later, with a superseding
  ADR.

Addendum (2026-11-06, IMPORT-013): a run can send its videos to a video destination, and
the log shows it (IMPORT-010, CLI-009), so the layout of section 3 changes for the first
time. Layout 2 adds the column `runs.video_destination` (a BLOB, NULL when the run has
none) and the index `copies_by_run` on `copies (run_id)`, which the import log reads by.
A new database is created with layout 2. Opened for writing, a database with
`user_version` 1 is migrated to 2 in one immediate transaction: the column and the index
are added and `user_version` is set to 2; existing rows keep their values, and their runs
have no video destination. A migration that fails leaves the database as it was and is
the error of section 6. Opened read-only for the import log, a database with layout 1 or
2 is read as it is, without migrating it. Any other `user_version` stays the error of
section 6. Later layouts migrate the same way, one step per version.

Addendum (2026-11-08, IMPORT-010.10): a read-only connection cannot remove the `-wal`
and `-shm` side files when it closes, so the import log left them in the state folder.
For the import log, `adapter-sqlite` therefore opens the database with a read-write
connection, without the flag that creates it, and sets `PRAGMA query_only`, which makes
every statement that would write fail. As the last connection to close, it removes the
side files, as an import does. A `-wal` file that a killed import left behind is then
checkpointed into the database: its committed rows become part of the file, which is
what the next import would do too. A history file the user cannot write is opened
read-only by SQLite and may still leave the side files. The rest of the addendum of
2026-11-04 stands: a missing database is not created, and a broken one or one with another layout
is the error of section 6.
