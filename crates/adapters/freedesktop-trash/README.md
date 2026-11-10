<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# adapter-freedesktop-trash

Layer: `adapter`, port `files` (architecture §3)

## Purpose

Implements the `Trash` port of `files` after the freedesktop.org Trash specification
1.0, in own code with `std` only (ADR 0043). GNOME, KDE and other Linux desktops show
and restore what it trashes. Remove RAWs and deleting in the library use it on Linux.

- **Which trash.** A file on the file system of the home trash goes there; the app
  passes the home trash folder, `$XDG_DATA_HOME/Trash`. A file on another file system
  goes to the trash at that file system's top folder: `.Trash/<uid>` if `.Trash` is a
  real folder with the sticky bit, else `.Trash-<uid>`. It is never copied across file
  systems; a trash that cannot be created is an error and the file stays.
- **Info file first.** `info/<name>.trashinfo` is created with `create_new`, which
  reserves the name, and synced before the file is renamed into `files/`. If the rename
  fails, the info file is removed again.
- **Names.** A name taken in `info` or `files` gets a number before its extension:
  `DSCF1234.RAF`, `DSCF1234.2.RAF`, …
- **Info content.** `Path` is absolute in the home trash and relative to the top folder
  elsewhere, percent-encoded; `DeletionDate` is the local time of the `Clock`, without a
  time zone.
- **Not covered here:** folders (the port refuses them, so no `directorysizes`),
  restoring and emptying the trash, which the desktop does.

## Public API

| Item | Meaning |
|---|---|
| `FreedesktopTrash` | `files::Trash` after the freedesktop.org Trash specification, with the home trash folder and a `domain::Clock` |

## Conformance

`tests/conformance.rs` runs `files::testing::check_trash` with a home trash in a
`TempDir`. `tests/trash_info.rs` checks the info file, numbered names and that a file
left in `files/` is never replaced. Unit tests cover the choice of the trash at a top
folder (sticky bit, symbolic link, private mode), the paths in the info file, the
percent-encoding and the names.

## Dependencies

- `domain`
- `files`
