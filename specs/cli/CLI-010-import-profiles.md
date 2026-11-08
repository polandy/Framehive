# CLI-010 – Import profiles in `framehive import`

- Status: implemented
- Module: cli (`apps/cli`)
- Origin: N8, N18, N25
- Depends on: CLI-001, CLI-005, IMPORT-011

## Purpose

`framehive import --profile holiday /run/media/ada/CARD ~/Pictures` imports a card with the
settings of a named import profile (IMPORT-011), so nobody repeats them on every run.
`framehive config init` writes a commented example to start from.

## Behaviour

```
framehive import <source>... <destination> [--profile <name>] [other arguments]
framehive import --destination <folder> <source>... [--profile <name>] [...]
framehive config init
```

**Configuration folder** (ADR 0032):

| Platform | Configuration folder |
|---|---|
| Linux | `$XDG_CONFIG_HOME/framehive`, or `$HOME/.config/framehive` when `XDG_CONFIG_HOME` is not set or not absolute |
| macOS | `$HOME/Library/Application Support/framehive` |

When `HOME` is not set or not absolute and the configuration folder needs it, a command
that needs the folder stops with exit code 2 and `error: the configuration folder is
unknown: set HOME to an absolute path`.

`--profile` also needs `HOME` for `~`; without an absolute one the command stops with
exit code 2 and `error: the home folder is unknown: set HOME to an absolute path`.

**Finding the profile.** `--profile <name>` reads `<configuration folder>/profiles/<name>.yaml`
as an import profile, with `HOME` as the home folder for `~`. A name that breaks the
rule of ADR 0032 (1 to 64 characters from `a`–`z`, `0`–`9`, `-`, `_`) is an invalid
argument. Every error in this section stops the command before anything is scanned,
with exit code 2 and one line on standard error:

| Situation | Message |
|---|---|
| Invalid name | `error: invalid profile name <name>: use 1 to 64 of a-z, 0-9, - and _` |
| No such file | `error: no profile <name> in <folder>; known profiles: <a>, <b>`, or `…; there are no profiles yet, see framehive config init` |
| File cannot be read | `error: the profile <path> cannot be read: <reason>` |
| Not UTF-8 | `error: the profile <path> is not valid UTF-8` |
| Invalid profile | `error: ` followed by the message of CORE-006 or IMPORT-011, with the file's full path as the file |

Known profiles are the files `*.yaml` in the profiles folder whose names are valid
profile names, sorted, without `.yaml`. Symbolic links count as files. A profiles folder that does not exist or
cannot be listed has no known profiles.

**Settings.** Each setting of the import comes from the first of: the option on the
command line, the profile, the default of CLI-001. An option given on the command line
replaces the profile's value, also for lists (ADR 0032):

| Setting | Option | Profile key |
|---|---|---|
| Template | `--template` | `naming.template` |
| Conflict policy | `--on-conflict` | `naming.on_conflict` |
| Job code | `--job-code` | the first of `job_codes` |
| Backup destinations | every `--backup` | `backup.destinations` |

The destination is never taken from a profile (CLI-016, IMPORT-011).

A job code taken from the profile is checked as in CLI-001. The warning of CLI-001 for a
job code that the template does not use is printed only for `--job-code`, not for a
profile's job codes. In the report of CLI-005, a backup destination from the profile is
shown as its path with `~` already replaced.

**`framehive config init`.** Writes the example profile
`<configuration folder>/profiles/example.yaml`, creating missing folders, prints
`wrote <path>` and exits with 0. The example has every key of IMPORT-011 and IMPORT-019,
each with a comment that explains it and names its default, and the
`yaml-language-server` line of IMPORT-011. When the file exists, nothing is written, standard error gets `error: <path>
exists already; nothing was written`, and the exit code is 1. A folder or the file that
cannot be written gives exit code 1 and `error: the folder <path> cannot be created:
<reason>` or `error: <path> cannot be written: <reason>`.

## Acceptance criteria

### CLI-010.1 – A profile supplies the settings
- **Given** the source of CLI-001.1, an empty folder `<destination>`, and a profile
  `holiday` with the template `{date:%Y%m%d}-{name}.{ext}` and an empty folder in
  `backup.destinations`
- **When** `framehive import <source> <destination> --profile holiday` runs
- **Then** the destination and the backup destination have `20261017-DSCF0001.RAF`,
  and the exit code is 0

### CLI-010.2 – An option replaces the profile's value
- **Given** the profile of CLI-010.1 and another empty folder `<other>`
- **When** `framehive import <source> <other> --profile holiday --template
  '{name}.{ext}'` runs
- **Then** `<other>` has `DSCF0001.RAF` and not the name by the profile's template, and
  the exit code is 0

### CLI-010.3 – `--backup` replaces the profile's list
- **Given** the profile of CLI-010.1 and another empty folder `<drive>`
- **When** the command of CLI-010.1 runs with `--backup <drive>`
- **Then** `<drive>` has the file, the profile's backup destination stays empty, and
  the report names only `<drive>`

### CLI-010.4 – The first job code of the profile
- **Given** a profile with the template `{job}/{name}.{ext}` and `job_codes: [Smith
  wedding, Jones]`
- **When** the command runs without `--job-code`, and then again with `--job-code
  Jones` into another empty destination
- **Then** the first destination has `Smith wedding/DSCF0001.RAF` and the second
  `Jones/DSCF0001.RAF`

### CLI-010.5 – Unused job codes of a profile give no warning
- **Given** a profile with the default template and `job_codes: [Smith wedding]`
- **When** the command runs
- **Then** standard error has no warning, and the exit code is 0

### CLI-010.6 – A profile with a destination
- **Given** a profile `holiday` with `destination`
- **When** `framehive import <source> <destination> --profile holiday` runs
- **Then** nothing is scanned, standard error is the error of CLI-016.13, and the exit
  code is 2

### CLI-010.7 – Unknown profile
- **Given** a profiles folder with `client.yaml` and `holiday.yaml`
- **When** the command runs with `--profile vacation`
- **Then** standard error has `error: no profile vacation in <folder>; known profiles:
  client, holiday`, and the exit code is 2

### CLI-010.8 – No profiles yet
- **Given** a configuration folder without a profiles folder
- **When** the command runs with `--profile holiday`
- **Then** the message ends with `there are no profiles yet, see framehive config init`,
  and the exit code is 2

### CLI-010.9 – Invalid profile name
- **Given** any configuration folder
- **When** the command runs with `--profile ../holiday` and with `--profile Holiday`
- **Then** each stops with exit code 2 and `error: invalid profile name …`, and no file
  is read

### CLI-010.10 – An invalid profile names the file and the position
- **Given** a profile `holiday.yaml` whose line 2 is `video_destinaton: /a`
- **When** the command runs with `--profile holiday`
- **Then** standard error has `error: <folder>/holiday.yaml:2:1: unknown key
  video_destinaton; did you mean video_destination?`, nothing is scanned, and the exit
  code is 2

### CLI-010.11 – A profile that is not UTF-8
- **Given** a profile `holiday.yaml` with the byte `0xFF`
- **When** the command runs with `--profile holiday`
- **Then** standard error has `error: the profile <path> is not valid UTF-8`, and the
  exit code is 2

### CLI-010.12 – The configuration folder on Linux
- **Given** Linux, `XDG_CONFIG_HOME` set to an absolute folder with the profile, and
  then `XDG_CONFIG_HOME` unset with the profile in `$HOME/.config/framehive/profiles`
- **When** the command runs with `--profile holiday`
- **Then** each run finds the profile

### CLI-010.13 – The configuration folder on macOS
- **Given** macOS and a profile in `$HOME/Library/Application Support/framehive/profiles`,
  with `XDG_CONFIG_HOME` set to another folder
- **When** the command runs with `--profile holiday`
- **Then** it finds the profile

### CLI-010.14 – Unknown configuration folder
- **Given** `XDG_CONFIG_HOME` not set and `HOME` set to `not-absolute`
- **When** the command runs with `--profile holiday`
- **Then** standard error has `error: the configuration folder is unknown: set HOME to
  an absolute path`, and the exit code is 2

### CLI-010.15 – `config init` writes the example
- **Given** an empty configuration folder
- **When** `framehive config init` runs
- **Then** `<folder>/profiles/example.yaml` exists, standard output has `wrote <path>`,
  the file is a valid import profile with the defaults of IMPORT-011 and IMPORT-019,
  every key of both appears in it, and the exit code is 0

### CLI-010.16 – `config init` keeps an existing example
- **Given** a `profiles/example.yaml` with the text `# mine`
- **When** `framehive config init` runs
- **Then** the file still holds `# mine`, standard error has `error: <path> exists
  already; nothing was written`, and the exit code is 1

### CLI-010.17 – A profile does not supply the destination
- **Given** the source of CLI-001.1 and the profile of CLI-010.1
- **When** `framehive import <source> --profile holiday` runs
- **Then** standard error is the error of CLI-016.2, and the exit code is 2

### CLI-010.18 – Unknown home folder with `XDG_CONFIG_HOME`
- **Given** Linux, an absolute `XDG_CONFIG_HOME` with the profile, and `HOME` not set
- **When** the command runs with `--profile holiday`
- **Then** standard error has `error: the home folder is unknown: set HOME to an absolute
  path`, and the exit code is 2

## Out of scope

- Profile keys and checks: IMPORT-011. Reading YAML: CORE-006.
- Profiles in the guided import: a later CLI-002 change.
- Binding a profile to a device (N8, Q9).
- Application settings and `framehive config` subcommands other than `init`.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-05 | Initial draft (N8, N18, N25, ADR 0032); accepted by the owner |
| 2026-11-05 | Implemented in `framehive-cli` |
| 2026-11-05 | Added CLI-010.18 (unknown home folder) |
| 2026-11-07 | Profiles have no destination (CLI-016, ADR 0036); CLI-010.1, .2, .6, .10 and .17 changed |
| 2026-11-08 | IMPORT-019 keys in the example |
