<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# Changelog

All user-visible changes are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow
[Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- Repository scaffold: Cargo workspace with the `domain` and `kernel` crates, pinned
  Rust toolchain, Nix development shell, and REUSE-compliant licensing
  (GPL-3.0-or-later).
- `cargo xtask ci` with the file, architecture, dependency and specification checks.
- Public API snapshots of `kernel` and the services: `cargo xtask check-public-api`
  fails on an API change without an updated `public-api.txt` and a changelog entry,
  and `cargo xtask public-api` writes the snapshots (ADR 0038).
- Reproducible release builds: `cargo xtask build-release` builds the `framehive`
  command with `SOURCE_DATE_EPOCH` and `--remap-path-prefix`, and
  `cargo xtask check-reproducible` in CI builds it twice from different directories
  and requires equal binaries on Linux (ADR 0007).
- Coverage-guided fuzzing of the EXIF parser: the fuzz crate in `fuzz/`, the
  `nix develop .#fuzz` shell with a pinned nightly toolchain, `cargo xtask fuzz` and a
  nightly workflow (ADR 0037).
- Name templates (IMPORT-001): the import plan gives each imported file its folders
  and name from a name template, keeps the files of one asset together, and marks
  files that cannot be named or are dated by their modification time.
- Import run: the import scans a source folder with all its subfolders and lists what
  it leaves out and why (IMPORT-005), dates each file by its capture time or else its
  modification time, resolves name conflicts and skips files already imported
  (IMPORT-003), treating names that differ only in case or Unicode normalization form
  as the same name (macOS and SMB shares may store names decomposed), and copies each
  file with a verified copy through a `.part` file (IMPORT-004). A file that fails is
  reported and the import goes on.
- `framehive import <source> --destination <folder>` (CLI-001): imports a folder or a mounted
  memory card into a library folder from the command line, with `--template`,
  `--on-conflict add_suffix|skip|fail` and `--dry-run`. It prints one line per file
  as soon as the file is done and a summary, creates a missing destination, rejects
  a destination inside the source, and exits with 0 (all done), 1 (a file failed or
  a folder could not be read) or 2 (the import did not start).
- Job codes (IMPORT-006): the `{job}` token of a name template inserts a job code,
  and `framehive import --job-code <text>` supplies it. A job code is trimmed and
  must have 1–64 characters, without `/` or control characters, and not dots only. A template with
  `{job}` and no job code, or an invalid job code, stops the command with exit code
  2; a job code the template does not use is accepted with a warning.
- Nix flake package: `nix run . -- import <source> --destination <folder>` runs the
  `framehive` command and `nix profile install .` installs it, built with the pinned
  toolchain.
- Guided import (CLI-002, CLI-003): `framehive` without a subcommand starts a line-based
  dialog in the terminal. It asks once for the library folder, waits for a memory
  card, offers three template presets with a preview of the card's first files, asks
  for a job code only when the template has `{job}`, and shows a plan summary before
  it imports with a progress line. It reports failed and skipped files, offers to
  eject the card, and stops between files when the card is pulled, going on when it
  is inserted again. The library folder, the template and the recent job codes are
  remembered in `$XDG_STATE_HOME/framehive` (IMPORT-006).
- Sequence numbers (IMPORT-002, CLI-004): name templates number assets in shooting
  order with `{seq_session:N}`, `{seq_letters}`, `{seq_today:N}` and
  `{seq_stored:N}`. The imported-today and stored counters are kept in
  `$XDG_STATE_HOME/framehive/sequence-counters.txt` and reserved when an import
  starts, so two imports never share a number; a dry run and the plan summary show the
  numbers without reserving them. Numbers that do not fit the template stop the
  import with a suggested template, and in the guided import lead back to the
  template step. Files imported before get no number (IMPORT-009).
- Backup destinations (IMPORT-007, CLI-005, CLI-006): `framehive import --backup <folder>`,
  repeatable, writes each copied or already imported file to the same path below every
  backup destination as well. The card is read once, every copy is verified on its
  own, and a backup target is never overwritten. The report has one line per backup
  destination for each file and a summary line for each; a failed backup makes the
  exit code 1, and a backup destination that is, contains or lies inside the
  destination, lies inside the source or is given twice stops the command. The guided
  import shows the backup destinations in the plan summary, adds or removes them with
  `b`, remembers them in the state file (now format version 2; version 1 is still
  read), refuses to start with one on the card, and reports failed backups.
- Format readiness (IMPORT-008, CLI-007): after an import, `framehive import` and the
  guided import say whether the source or card can be formatted, that is whether every
  file is verified at the destination and at every backup destination. Otherwise they
  list each file that is not saved, with the number of locations it is verified at, and
  each unreadable or ungrouped entry the scan left out. The guided import shows it before
  the eject question; a dry run shows nothing, and exit codes are unchanged.
- Import history in the import run (IMPORT-009): an import recognizes files it copied
  before by name, size, capture time and checksum, even after they were renamed or
  deleted in the library or the template changed. It does not copy them again, gives
  them no sequence number, reports them as `imported before`, and completes their
  missing backups. Each verified copy is recorded. `framehive import` and the guided
  import keep the history in `import-history.sqlite` in the state folder (CLI-008,
  ADR 0031), so a card imported again is recognized across runs, and an interrupted
  import is resumed by running it again. A history that cannot be used stops the
  import before any file is copied.
- `framehive import-log [<run>|last] [--format text|json|csv]` (CLI-009, IMPORT-010):
  lists the recorded import runs, oldest first, or prints which file of one run went
  where, with its checksum, as text, JSON or RFC 4180 CSV to redirect into a file. It
  reads the import history without creating or changing it.
- Import profiles (IMPORT-011, CLI-010, ADR 0032): `framehive import <source>
  <destination> --profile <name>` takes the backup destinations, the name template,
  the conflict policy and the job code from the YAML file `profiles/<name>.yaml` in the
  configuration folder (`$XDG_CONFIG_HOME/framehive` or `~/.config/framehive` on Linux,
  `~/Library/Application Support/framehive` on macOS). An option given on the command
  line replaces the profile's value, also for lists. A profile is read strictly: an
  unknown key, a wrong type or an invalid value stops the command with the file, line
  and column. `framehive config init` writes a commented example,
  `profiles/example.yaml`, and never replaces an existing one. The JSON Schema
  `schemas/import-profile-v1.json` gives completion in editors.
- Free-space check (IMPORT-012, CLI-011, ADR 0033): `framehive import` and the guided
  import check, before the first folder is created, that each file system can hold
  what the import writes to the destinations on it, counting only the files that are
  not there yet. A short file system is reported with what it needs and has free, and
  the import does not start (exit code 2; in the guided import, Enter does not start
  until `d` or `b` changes a folder). A dry run checks too. When the free space cannot
  be read, a warning says so and the import goes on.
- Video destinations (IMPORT-013, CLI-012): `framehive import --video-destination
  <folder>` and the profile key `video_destination` send videos to a folder of their own,
  and `--video-backup <folder>` (repeatable) and `backup.video_destinations` give them
  their own backup destinations; unset, videos go where photos go. Each video line of
  the report ends with `(video destination)`, the format readiness counts each file's
  own locations, and `framehive import-log` shows a run's video destination. The import
  history moves to layout 2 the first time it is written (ADR 0031).
- Cameras and phones over PTP and MTP (IMPORT-014, CLI-013, ADR 0034):
  `framehive import camera: --destination <folder>` imports from the only connected camera, and
  `camera:<model>` or `camera:<port>` picks one of several; `framehive cameras` lists
  the connected cameras by model and port. Only the `DCIM` folder of each storage is
  scanned; the camera is read through libgphoto2, never written, and released when the
  command ends. A camera that another program holds (the file manager on Linux,
  `ptpcamerad` on macOS) stops the import with exit code 2 and says what to close. A
  camera that shows no storage, such as a locked phone or one set to charge only, also
  stops it with exit code 2 and says to unlock it and choose file transfer.
  After the format readiness, a camera run notes that only the files below `DCIM` were
  checked. The import log names the source `camera:<model>`. A folder whose name starts
  with `camera:` is given as `./camera:…`.
- Cameras in the guided import (CLI-014): `framehive` without a subcommand waits for a
  memory card or camera and offers a camera or phone connected over PTP or MTP like a
  card, by its model. A camera that another program holds or that shows no storage
  says what to do and asks `Try again?`. After the import the dialog asks
  `Release <model>?`. A camera disconnected during the import, or one that stops
  answering, stops the import like a pulled card; connected again, it is imported
  where it stopped without a question.
- Job queue (CORE-007, IMPORT-015, ADR 0035): imports now run as jobs in a job queue
  with one worker thread, first in, first out. `framehive import` and the guided import
  show the same output as before; a pulled card or camera still stops the import.
- Several sources in one command (CLI-015, IMPORT-015, ADR 0036):
  `framehive import <source>... --destination <folder>` imports each source as a job of its
  own. Jobs whose sources are on separate devices and whose destinations share no disk
  run at the same time; the others run one after another. Each job reserves its
  sequence numbers and target names when it is queued, so parallel imports never take
  the same ones. With several sources, each summary follows a line `<source>:`, and
  the exit code is the worst of the sources. A dry run of several sources counts the
  numbers and target names across them in memory (IMPORT-016), so it shows what the
  real run would do, and stores nothing.
- Video name template (IMPORT-018, CLI-012): `framehive import --video-template
  <template>` and the profile key `naming.video_template` name videos by a template of
  their own; unset, videos use the name template. Photos and videos share the conflict
  policy, the job code and one numbering; when both templates have a sequence token,
  they must use the same kind, else the import does not start. The naming service's
  `preview_range` and `reserve_range` take every template of an import, and
  `SequenceKind::token` names a kind in messages.
- XMP packets (CORE-009.1–.14, ADR 0039): `metadata::xmp` reads every RDF form of
  ISO 16684-1 into one model, edits it without losing unknown fields, and writes
  Framehive's canonical form with `xmp:MetadataDate`; `adapter-xmp` provides the XML
  tokens with `quick-xml`. Document type declarations, other encodings than UTF-8 and
  conflicting values are refused. A mutation test and the fuzz target `xmp` check it.
- XMP sidecar files (CORE-009.15–.21, ADR 0040 §1, §7): `metadata::xmp` names sidecars
  as `DSCF0042.RAF.xmp` or `DSCF0042.xmp`, finds both next to a file, and merges into a
  sidecar through a `.part` file and a rename that never replaces a sidecar another
  program created or changed meanwhile, a read-only one, or one it cannot parse. The
  `files` port gains `replace_file`, which keeps the replaced file's permissions.
- Hierarchical keywords in XMP (ADR 0040 §2): `XmpPacket::add_keywords` adds each
  keyword's leaf to `dc:subject`, its levels joined by `|` to `lr:hierarchicalSubject`
  and its path to the `mwg-kw:Keywords` tree, merging levels under the same parent.
  `metadata::testing::RecordedXml` is an `XmlSyntax` test double.
- The import history records a copy's sidecar (`history::SidecarRecord`; IMPORT-019,
  ADR 0031) and moves to layout 3 the first time it is written.
- Metadata at import (IMPORT-019, ADR 0040): an import with a job code or `metadata`
  values writes an XMP sidecar next to each imported photo, with the job code as
  `photoshop:TransmissionReference`, the creators, the copyright with `{year}` and the
  keywords. It is written after the verified copy, read back and compared, written the
  same to each backup, and recorded in the import history. Its name is kept free with
  the file's, so a suffix goes to both and a leftover sidecar is never merged with. A
  sidecar that cannot be written fails its file, which keeps the card from being
  formatted; a re-run that finds the file already imported writes its missing sidecar
  and keeps one that is there. A dry run lists the sidecars it would write.
  `files::testing::MemoryFileSystem` gains `remove_fault`.
  `metadata::xmp` gains `FoundSidecar::nothing` for a sidecar written new and
  `SidecarErrorKind::Differs` for one that does not read back as written.
- Source sidecars (IMPORT-020, ADR 0040 §4, §5): a `.xmp` sidecar on the card comes
  with the file it names and keeps its naming form. Without import values it is copied
  byte for byte; with values, the job code replaces the old one, creators and a
  copyright already there are kept, keywords are added, and unknown fields stay. The
  summary lists each replaced and kept value. A `.xmp` without its file is left out as
  "sidecar without its file", and a sidecar counts as saved only where it was written
  and verified, so either keeps the card from being formatted.
- Sidecars in the report (CLI-017): `framehive import` prints `wrote sidecar` (in a dry
  run `would write sidecar`) under each file that got one, a `replaced` or `kept` line
  for each value a sidecar from the card already had, and `<n> sidecars written` after
  the summary. The guided import shows the creator, copyright and keywords in its plan
  summary and reports the replaced and kept values and the sidecar count.

### Changed

- `framehive config init` writes the example profile with `schema_version: 2` and the
  `metadata` keys of IMPORT-019 as comments (CLI-010).
- `framehive import` names its sources and destination as `cp` does (CLI-016,
  ADR 0036): `framehive import <source>... <destination>` keeps working, and the new
  `framehive import --destination <folder> <source>...` makes every folder a source.
  A command without a source or without a destination, or with a camera where the
  destination belongs, stops with exit code 2 and an example.
- **Breaking:** an import profile no longer has the key `destination`; the folder to
  import into is always given on the command line, so a folder's meaning never
  depends on `--profile`. A profile that still has it stops the command with exit
  code 2 and a message at its line (IMPORT-011).

### Fixed

- The free-space check before an import counts the XMP sidecars it writes, so an
  import that would fill the destination with its sidecars no longer starts
  (IMPORT-012).
- `framehive import-log` no longer leaves the `-wal` and `-shm` files of SQLite next to
  the import history in the state folder (IMPORT-010).
- After an import that found no file, the format readiness no longer counts the video
  backup destinations as locations: with one backup destination and a video backup
  destination of its own, it says "verified in 2 locations", the import destination and
  the backup destination a photo would need (IMPORT-008, IMPORT-013).
- `framehive import` and the guided import no longer create the destination folders
  of an import that does not start because its job, when it starts, finds too little
  free space, for example after another import to the same disk filled it. The import
  run now creates the missing destinations itself, after its free-space check
  (IMPORT-012, CLI-011). `framehive import` reports that shortage with the sizes and
  folders as given, as its first check does.
- Two `framehive config init` commands at the same time no longer damage the example:
  each one writes its own `.part` file, so the second cannot empty the file the
  first one just put in place (CLI-010).
- `framehive import` and the guided import now warn when the import's own check, as
  its job starts, cannot read the free space of a destination that the first check
  could read: `cannot tell the free space at <folder>: <reason>` follows the import's
  summary. A destination is named once (CLI-011).
- A configuration file with `schema_version:` but no value now says that the key needs
  a positive whole number; before, it suggested leaving the required key out (CORE-006).
- `framehive import-log` keeps its text on one line per run and per copy when a path or
  target name contains a line break or another control character: it prints them
  escaped, such as `\n`, as the JSON format does (CLI-009).
- Files already imported are recognized when the destination tells case or Unicode
  normalization forms apart and holds the same name in several spellings, such as
  `Café.JPG` precomposed and decomposed: every spelling is compared, so a photo
  identical to any of them is no longer copied again with a suffix (IMPORT-003).
- Files already imported are recognized when their target folder exists at the
  destination in another case or Unicode normalization form, such as `CAFÉ/` for
  `Café/`, at a destination that tells such names apart (IMPORT-003).
- A file where a folder of a target path should be is found when the import is
  planned: the dry run no longer says it would copy, and both the dry run and the run
  fail the photo with "the target folder <path> is a file" instead of "<folder> does
  not exist". A photo no longer takes the name of a video destination inside the
  import destination that a dry run has not created (IMPORT-017).
