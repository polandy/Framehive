# Where Framehive keeps its files

Besides the photos where you import them, Framehive keeps its files in two folders.

## Configuration folder

Your settings: `config.yaml` and the import profiles in `profiles/`
([Configuration](configuration.md)). Framehive only reads it, except for
`framehive config init`.

| Platform | Folder |
|---|---|
| Linux | `$XDG_CONFIG_HOME/framehive`, or `~/.config/framehive` when `XDG_CONFIG_HOME` is not set or not absolute |
| macOS | `~/Library/Application Support/framehive` |

## State folder

What Framehive remembers between runs: `$XDG_STATE_HOME/framehive`, or
`~/.local/state/framehive` when `XDG_STATE_HOME` is not set or not absolute, on Linux and
macOS.

| File | What it holds |
|---|---|
| `import-history.sqlite` | Every verified copy: source, destination, name, size, capture time and checksum. `framehive import-log` shows it; a re-run uses it to skip files imported before |
| `sequence-counters.txt` | The counters of `{seq_today:N}` and `{seq_stored:N}` |
| `guided-import.txt` | The last answers of the guided import: destination, backups, template and recent job codes |

`sequence-counters.lock` next to the counters keeps two imports from taking the same
number. `framehive import` stops with exit code 2 when it cannot find the state folder,
also in a dry run. Deleting `import-history.sqlite` loses the record of earlier imports
that `framehive import-log` shows and that a re-run uses. Deleting
`sequence-counters.txt` starts the remembered running numbers again at 1.

## Trash

`framehive library remove-raws` moves files to the trash of the system, never deletes
them: on Linux the trash of the desktop (`$XDG_DATA_HOME/Trash`, usually
`~/.local/share/Trash`, or for a file on another disk the trash at that disk's top
folder, `.Trash/<uid>` or `.Trash-<uid>`), on macOS the
Finder's Trash. Restore a file from there with the file manager.
