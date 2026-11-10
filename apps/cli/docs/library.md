<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# framehive-cli: library commands

`framehive library keep-raw` and `framehive library remove-raws` (CLI-018) put the Keep
RAW marker and Remove RAWs of the `library` module (LIBRARY-002) on the command line
until the GUI has them. `src/library_command/` holds them:

- `mod.rs` wires the adapters: `adapter-fs`, `adapter-blake3`, `adapter-xmp` and
  `adapter-clock` behind `metadata::xmp::SidecarFiles`, the sidecar naming of
  `config.yaml`, and the trash of the system: `adapter-freedesktop-trash` with the home
  trash in `$XDG_DATA_HOME/Trash` (or `~/.local/share/Trash`) on Linux,
  `adapter-macos-trash` on macOS (ADR 0043). Paths are made absolute first, so every
  line names a file in full. It also writes the warnings for unreadable sidecars and
  ambiguous base names that `library::read_folder` returns.
- `keep_raw.rs` reads the folder of each named file once, finds each file's asset,
  stops with exit code 2 before any change when a file is not part of one, and then
  calls `library::set_keep_raw` per asset.
- `remove_raws.rs` prints the summary of `library::RemoveRawsPlan`, asks on standard
  input, runs the plan on the trash and reports each file that did not move.

Both functions take the ports and the streams, so `src/library_command/tests.rs` runs
them on a `MemoryFileSystem` with a `MemoryTrash` that can refuse a file (CLI-018.4,
.6, .8, .10). `tests/library.rs` runs the binary with `XDG_DATA_HOME` below a temporary
folder; the confirmed run is not run against the binary on macOS, where it would use
the Trash of the user who runs the tests.
