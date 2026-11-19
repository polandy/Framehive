# User and administrator documentation

Guides for people who install, configure and use Framehive. Technical documentation
for developers is in [`../dev/`](../dev/README.md).

Framehive is at an early stage: what runs today is the command line. It imports photos
and videos from memory cards, folders and cameras, and it frees space by moving the RAW
of RAW + JPEG pairs to the trash. The graphical library comes later.

| Guide | What it covers |
|---|---|
| [Installation](installation.md) | Installing with Nix, building from source, cameras on Linux |
| [Getting started](getting-started.md) | The first import, from a memory card and from a folder |
| [Commands](commands.md) | Every command and option, name templates, the report and the exit codes |
| [Configuration](configuration.md) | `config.yaml`, import profiles and their keys |
| [Where Framehive keeps its files](locations.md) | Configuration, state, import history and trash |
| [Safety questions](safety.md) | When a card can be formatted, backups, Remove RAWs |

`framehive --help` and `framehive <command> --help` print the options in short.
