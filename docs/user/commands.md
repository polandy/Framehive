# Commands

Every command accepts `-v` / `--verbose`, before or after the command name. Once shows
what the command does, such as each file of an import; twice adds details, such as a
rename without a hard link. These diagnostic lines go to standard error only, so the
report and the exit codes stay the same; attach them to an error report.

| Command | What it does |
|---|---|
| `framehive` | Guided import of a memory card or camera, step by step in a terminal |
| `framehive import` | Import folders, cards or cameras without questions |
| `framehive import-log` | Show the import history |
| `framehive cameras` | List the cameras and phones connected over PTP or MTP |
| `framehive config init` | Write the example import profile |
| `framehive library keep-raw` | Mark assets Keep RAW, so Remove RAWs keeps their RAW |
| `framehive library remove-raws` | Move the RAW of RAW + JPEG pairs to the trash |

## `framehive import`

```
framehive import [OPTIONS] <source>... <destination>
framehive import [OPTIONS] --destination <folder> <source>...
```

A source is a folder, a mounted card, or a camera: `camera:` for the only connected
camera, `camera:<model>` or `camera:<port>` to pick one (`framehive cameras` lists
them; `./camera:x` is a folder of that name). Several sources are imported in the order
given, in parallel where they share no disk. The destination is created with its
parents when it is missing; it must not lie inside a source.

| Option | Meaning | Default |
|---|---|---|
| `--destination <folder>` | Folder to import into; then every positional argument is a source | the last positional argument |
| `--backup <folder>` | Also copy each file here and verify it; repeat for more backups | none |
| `--video-destination <folder>` | Folder for videos | the destination |
| `--video-backup <folder>` | Backup folders for videos, instead of `--backup`; repeatable | the `--backup` folders |
| `--template <template>` | Folders and name of each file, see below | `{date:%Y}/{date:%Y-%m-%d}/{name}.{ext}` |
| `--video-template <template>` | Name template for videos | the name template |
| `--job-code <text>` | Text for the `{job}` token | none |
| `--on-conflict <policy>` | When a name holds a different file: `add_suffix` (`DSCF0001-1.RAF`), `skip` or `fail` | `add_suffix` |
| `--profile <name>` | Use the settings of an [import profile](configuration.md); options given here win | none |
| `--dry-run` | Make the plan and print it; copy nothing, reserve no numbers | off |

A job code has 1 to 64 characters, no `/`, no control characters, and is not only dots.

### Name templates

A template is a path with `/` between folders; text outside braces is kept as it is,
`{{` and `}}` are literal braces, and the last part must end with `.{ext}`.

| Token | Value |
|---|---|
| `{date:<format>}` | Capture time of the photo or video |
| `{imported:<format>}` | Time the import was queued, in local time |
| `{name}`, `{name:lower}`, `{name:upper}` | Original name without extension |
| `{ext}`, `{ext:lower}`, `{ext:upper}` | Original extension without the dot |
| `{number}`, `{number:N}` | Image number: the trailing digits of the original name, the last N of them (1–6) |
| `{seq_today:N}` | Running number of all imports of the day, N digits, remembered |
| `{seq_stored:N}` | Running number of all imports ever on this machine, remembered |
| `{seq_session:N}`, `{seq_letters}` | Running number of this run only, as digits or `a`, `b`, …, `aa` |
| `{job}` | The job code |
| `{camera}` | Camera model from the photo's EXIF |

A template holds at most one running number. Date formats use `%Y` (2026), `%y` (26),
`%m`, `%d`, `%j` (day of the year), `%H`, `%M`, `%S` and `%%`, the same on every machine.
A RAW and its JPEG get the same values, and a sidecar follows its file.

### The report

One line per file as soon as it is done: `copied`, `already imported` (identical file
at the target), `imported before` (in the import history), `duplicate` (same content as
an earlier file of this run), `skipped`, `failed` or `left out` (not a photo or video,
hidden, a link, or unreadable). Under each file follows one line per backup folder.
A summary closes the report, and after a real import the answer whether the source can
be formatted ([Safety questions](safety.md)).

### Exit codes

| Code | Meaning |
|---|---|
| 0 | Every file is saved, or was skipped or left out on purpose |
| 1 | The import ran, but a file or a backup failed, a folder could not be read, or the import history failed during the run |
| 2 | The import did not start: invalid arguments, template, job code or profile, a missing source, a destination that is inside the source or cannot be created, not enough free space, or a state folder or import history that cannot be used |

With several sources, the exit code is the highest of the sources.

## `framehive import-log`

```
framehive import-log                      # one line per run
framehive import-log <run|last> [--format text|json|csv]
```

The log of a run names each file, where it went and its checksum. It reads the history
only and never changes it. Exit code 2 when the run or the history does not exist.

## `framehive cameras`

Prints `<model>  <port>` per connected camera or phone. When none is listed, see
[Cameras on Linux](installation.md#cameras-on-linux).

## `framehive config init`

Writes `profiles/example.yaml` into the [configuration folder](locations.md). An
existing file is kept, and the exit code is then 1.

## `framehive library keep-raw`

```
framehive library keep-raw [--off] <file>...
```

Marks the asset of each file (a photo, its RAW, JPEG or sidecar) Keep RAW, or unmarks it
with `--off`. The marker is stored in the asset's XMP sidecar. When a file is not part of
an asset, nothing changes and the exit code is 2.

## `framehive library remove-raws`

```
framehive library remove-raws [--dry-run] [--yes] <folder>
```

Looks at the assets of one folder, without its subfolders. For each asset with a RAW
and a rendered file (such as a JPEG) that is neither rejected nor marked Keep RAW, it
moves the RAW and the RAW's own sidecar to the trash; the JPEG and its metadata stay.
It shows the totals first and asks `[y/N]`; `--yes` skips the question for scripts, and
`--dry-run` lists the files and moves nothing. Without an answer (an input that ends),
nothing moves and the exit code is 2. Right before moving, it looks at the folder again:
an asset whose JPEG or RAW is gone, or that was marked Keep RAW or rejected meanwhile,
keeps its RAW and is named in a warning. The exit code is 1 when a file did not move or
an asset changed since the summary.
