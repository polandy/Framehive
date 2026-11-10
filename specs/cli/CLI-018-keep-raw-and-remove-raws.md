# CLI-018 – Keep RAW and Remove RAWs on the command line

- Status: implemented
- Module: cli (`apps/cli`)
- Origin: B29, B30
- Depends on: LIBRARY-001, LIBRARY-002, CLI-001

## Purpose

Until the library has a GUI, `framehive library keep-raw` marks assets Keep RAW and
`framehive library remove-raws` frees the space of all other RAWs of a folder, with the
rules of LIBRARY-002 and a text summary to confirm (owner decision A on #109).

## Behaviour

```
framehive library keep-raw <file>... [--off]
framehive library remove-raws <folder> [--dry-run]
```

**Assets.** Both commands group the files directly in a folder into assets
(LIBRARY-001) and read each asset's Keep RAW marker and rejected state from its
sidecars. Before anything else they write to standard error:

| Line | When |
|---|---|
| `warning: the XMP sidecar <sidecar> <reason>; its asset is left out` | A sidecar of the folder cannot be read |
| `warning: <folder>/<base>.*: more than one RAW or rendered file; each is an asset of its own` | A base name does not group (LIBRARY-001.4) |

**`keep-raw`.** Each `<file>` is a media file of an asset, or one of its sidecars. The
command marks each named asset Keep RAW, or unmarks it with `--off`, in the sidecar of
every member (LIBRARY-002.7), with the sidecar naming of `config.yaml` (ADR 0040) for a
member that has none. An asset named by several files is changed once. Standard output
gets one line per asset, in the order of the command line:

```
Keep RAW on: <folder>/<base>
Keep RAW off: <folder>/<base>
```

A file that is not part of a readable asset stops the command before any sidecar
changes: `error: <file> is not a photo or video of the library`, exit code 2. An asset
whose marker cannot be set (LIBRARY-002.6, or a sidecar that cannot be written) gets
`error: cannot set Keep RAW of <folder>/<base>: <reason>` on standard error; the other
assets are changed and the exit code is 1.

**`remove-raws`.** The view is every asset of `<folder>`. Standard output gets the
summary of LIBRARY-002, with base names in name order, `-` for an empty group, and
sizes as the import writes them (`30.0 MB`, `12.8 GB`) and `asset` and `file` in the
singular for one:

```
Remove RAWs: <assets> assets, <files> files, <size> freed
  RAW will be removed (<count>): <base>, <base>, …
  RAW stays (Keep RAW) (<count>): <base>, …
  Skipped: <n> rejected, <n> JPEG only, <n> RAW only
```

With `--dry-run`, the files that would go to the trash follow, one per line indented by
four spaces, and nothing moves. Otherwise, when no file would go, `Nothing to remove.`
follows. Else the command asks `Move <files> files to the trash? [y/N] ` on standard
output and reads one line from standard input: `y` or `yes`, in any case, moves the
files; anything else, or the end of input, prints `Nothing moved.` and moves none.

The files move asset by asset to the trash of the system: on Linux the freedesktop.org
trash, with the home trash in `$XDG_DATA_HOME/Trash`, or `~/.local/share/Trash` when it
is not set or not absolute; on macOS the Trash, through Finder's call (ADR 0043). A file
that cannot be moved gets `error: cannot move <file> to the trash: <reason>` on
standard error, with the failure rules of LIBRARY-002.16 and .17. Then standard output
gets:

```
Moved to the trash: <assets> assets, <files> files, <size> freed
```

counting only what moved. The exit code is 1 when a file did not move, else 0. A folder
that cannot be read stops the command with `error: <message>` and exit code 2, as does
an unknown home folder on Linux.

## Acceptance criteria

### CLI-018.1 – keep-raw marks every member
- **Given** `A.RAF` and `A.JPG` without sidecars, and the default sidecar naming
- **When** the user runs `framehive library keep-raw A.JPG`
- **Then** `A.RAF.xmp` and `A.JPG.xmp` hold `framehive:KeepRaw` `True`, standard output
  is `Keep RAW on: <folder>/A`, and the exit code is 0

### CLI-018.2 – --off unmarks
- **Given** the asset of .1, marked
- **When** the user runs `framehive library keep-raw A.RAF --off`
- **Then** neither sidecar holds `framehive:KeepRaw`, and standard output is
  `Keep RAW off: <folder>/A`

### CLI-018.3 – A file outside the library changes nothing
- **Given** a marked asset `A` and a file `notes.txt`
- **When** the user runs `framehive library keep-raw A.RAF notes.txt --off`
- **Then** standard error has `error: <folder>/notes.txt is not a photo or video of the
  library`, `A` stays marked, and the exit code is 2

### CLI-018.4 – A refused marker stops only its asset
- **Given** assets `A` and `B`, and `A.RAF.xmp` holds `framehive:KeepRaw` as an array
- **When** the user runs `framehive library keep-raw A.RAF B.RAF`
- **Then** standard error names `A` with the reason, `B` is marked, and the exit code
  is 1

### CLI-018.5 – The summary
- **Given** `A` (RAF 30 MB + JPG, sidecar `A.RAF.xmp`), `B` (RAF+JPG, marked), `C` (JPG
  only), `D` (RAF only) and `E` (RAF+JPG, rejected)
- **When** the user runs `framehive library remove-raws <folder> --dry-run`
- **Then** standard output shows 1 asset, 2 files and the size of `A.RAF` and
  `A.RAF.xmp`; *RAW will be removed* (1): `A`; *RAW stays (Keep RAW)* (1): `B`;
  skipped 1 rejected, 1 JPEG only, 1 RAW only; then `A.RAF` and `A.RAF.xmp`; and no
  file moved

### CLI-018.6 – Confirming moves the RAWs
- **Given** the folder of .5
- **When** the user runs `framehive library remove-raws <folder>` and answers `y`
- **Then** `A.RAF` and `A.RAF.xmp` are in the trash, every other file stays, standard
  output ends with `Moved to the trash: 1 asset, 2 files, <size> freed`, and the exit
  code is 0

### CLI-018.7 – Any other answer moves nothing
- **Given** the folder of .5
- **When** the user answers `n`, or standard input ends
- **Then** standard output ends with `Nothing moved.`, no file moved, and the exit code
  is 0

### CLI-018.8 – A file that cannot be moved is reported
- **Given** unmarked RAF+JPG assets `A` and `B` with own sidecars, and `A.RAF` cannot be
  moved to the trash
- **When** the user confirms Remove RAWs
- **Then** standard error has `error: cannot move <folder>/A.RAF to the trash:
  <reason>`, `A.RAF` and `A.RAF.xmp` stay, `B`'s two files are in the trash, the totals
  count 1 asset and 2 files, and the exit code is 1

### CLI-018.9 – Nothing to remove
- **Given** a folder with only `C.JPG`
- **When** the user runs `framehive library remove-raws <folder>`
- **Then** the summary shows 0 assets and `-` for both groups, `Nothing to remove.`
  follows, the command asks nothing, and the exit code is 0

### CLI-018.10 – An unreadable sidecar keeps its asset out
- **Given** an unmarked RAF+JPG asset `A` whose `A.JPG.xmp` is not XMP, and an unmarked
  RAF+JPG asset `B`
- **When** the user runs `framehive library remove-raws <folder> --dry-run`
- **Then** standard error warns that `A.JPG.xmp` cannot be read, and only `B` is in
  *RAW will be removed*

## Out of scope

- Key `R`, the filter, the index and the summary with thumbnails, which toggle the
  marker by a click (LIBRARY-002.1, .8, .13, .14): the GUI.
- Folders below `<folder>`, and a view narrowed by filter or search.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-10 | Initial version, owner decision A on #109 |
