<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# library

Layer: `module` (architecture §3, §4)

## Purpose

The library module works on the photos already in the library. It groups the files of
a folder into assets by their names alone (LIBRARY-001): one RAW and one rendered file
with the same base name form one asset, a video is an asset of its own with its
`.THM`, and an `.xmp` sidecar joins the file it is named after or, as a shared
`<base>.xmp`, the asset of that base name. A base name with two RAWs or two rendered
files is reported as ambiguous, and each of its files becomes an asset.

`read_folder` reads each asset's Keep RAW marker and rejected state (ADR 0044) back
from its sidecars, so a rebuild gives the same result. The most recently modified
sidecar holds the state, and on a tie the RAW's own one. An asset with an unreadable
sidecar is left out and its sidecar reported, so no command acts on unknown state.

`set_keep_raw` writes the marker into the sidecar of every media file of an asset
(LIBRARY-002.7), creating one where none exists. It checks every sidecar first, so a
marker it cannot set changes no file.

`RemoveRawsPlan` decides which RAWs of a view go to the Trash (LIBRARY-002): the RAW
of each asset with a RAW and a rendered file that is neither marked Keep RAW nor
rejected, with the RAW's own sidecars. A shared sidecar stays. It counts the skipped
assets, gives the totals, and `toggle_keep_raw` moves an asset between the groups by
writing its marker first. `run` moves the files through the `files::Trash` port; a RAW
that cannot be moved keeps its sidecars, and every file that was not moved is reported
with the reason.

The module touches files only through the `files` ports and sidecars only through
`metadata`. `framehive library keep-raw` and `framehive library remove-raws` use it
(CLI-018); key `R`, the filter, the index and the thumbnail summary wait for the GUI.

## Public API

| Item | Meaning |
|---|---|
| `MODULE_ID`, `module_id` | Stable id of the module (architecture §4.2) |
| `group_files`, `Grouping`, `FolderFile`, `Asset`, `AssetFile`, `AmbiguousBaseName` | The grouping rule (LIBRARY-001) |
| `read_folder`, `LibraryFolder`, `LibraryAsset`, `ReadFolderError` | A folder's assets with their Keep RAW marker and rejected state |
| `set_keep_raw` | Writes the Keep RAW marker into every member's sidecar (LIBRARY-002.7) |
| `RemoveRawsPlan`, `SkippedAssets`, `RemoveRawsTotals`, `RemoveRawsReport`, `NotMoved` | Remove RAWs: the plan, the toggle and the run (LIBRARY-002) |
| `ExampleViewModel` | Example ViewModel from the template; replace it |

## Events

Published: none yet. Subscribed: none yet. New events need an ADR (architecture §4.5).

## Commands

Handled: none yet. Sent: none yet. New commands need an ADR (architecture §4.5).

## Data

No tables yet. Tables use the prefix `library_` and describe their rebuild
(architecture §4.4).

## Extension points

None yet.

## Dependencies

- `domain`: asset file roles and the RAW and video extensions
- `files`: the file system and `Trash` ports
- `kernel`
- `metadata`: XMP sidecars, Keep RAW and the rejected state
- `thiserror`
