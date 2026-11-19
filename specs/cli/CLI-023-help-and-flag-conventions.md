# CLI-023 – Help and flag conventions

- Status: implemented
- Module: cli (`apps/cli`)
- Origin: N18
- Depends on: CLI-001, CLI-018

## Purpose

`framehive --help` lists the commands, but a new user also needs an example, the
folders Framehive keeps its files in and what the exit codes mean. And a user who wants
to try a command that changes files needs a run that changes nothing (review E27).

## Behaviour

**Help.** Below the commands and options, `framehive --help` shows three sections:

```
Examples:
  framehive                                 Import a memory card, guided by questions
  framehive import /media/card ~/Pictures   Import a card into ~/Pictures
  framehive import-log last                 Show where each file of the last run went

Folders:
  Configuration: <configuration folder>
  State and import history: $XDG_STATE_HOME/framehive, or ~/.local/state/framehive

Exit codes:
  0  Done
  1  The command ran, but a file or folder failed
  2  The command did not start: invalid arguments, or a missing or unusable folder
```

The configuration folder is the one of the platform it runs on:
`$XDG_CONFIG_HOME/framehive, or ~/.config/framehive` on Linux, `~/Library/Application
Support/framehive` on macOS (CLI-010).

**Flags.** Flags are long full words; `-h`, `-v` and `-V` are the only short ones.
Every command outside the guided import that changes files has `--dry-run`: it checks
the arguments and the files as a real run does, prints what would change and changes
nothing. A command that asks before it changes files has `--yes` (CLI-018).

**`keep-raw --dry-run`.** `framehive library keep-raw <file>... --dry-run` finds the
assets as CLI-018 does, with the same errors and exit code 2, writes no sidecar, and
prints one line per asset with `would be`:

```
Keep RAW would be on: <folder>/<base>
Keep RAW would be off: <folder>/<base>
```

A dry run does not try to write, so a marker that cannot be set (CLI-018.4) shows only
in the real run.

## Acceptance criteria

### CLI-023.1 – The help shows examples, folders and exit codes
- **When** the user runs `framehive --help`
- **Then** standard output has the sections `Examples:`, `Folders:` and `Exit codes:`,
  with `framehive import /media/card ~/Pictures`, the configuration folder, the state
  folder with the import history, and the meaning of 0, 1 and 2, and the exit code is 0

### CLI-023.2 – keep-raw --dry-run changes nothing
- **Given** `A.RAF` and `A.JPG` without sidecars
- **When** the user runs `framehive library keep-raw A.JPG --dry-run`
- **Then** standard output is `Keep RAW would be on: <folder>/A`, no sidecar exists,
  and the exit code is 0

## Out of scope

- A manual page and shell completions: later, with packaging.
- The help of each subcommand: CLI-001 and the spec of each command.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-19 | Initial version, implemented (review finding E27) |
