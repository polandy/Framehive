# IMPORT-011 – Import profiles

- Status: implemented
- Module: import
- Origin: N8, N25, R9, R7, I4
- Depends on: CORE-006, IMPORT-001, IMPORT-003, IMPORT-006, IMPORT-007, IMPORT-013

## Purpose

A photographer imports the same way again and again: the same backup drive, the same
template, the same job codes. An import profile keeps these settings under a name,
in a YAML file the user can read, edit, copy to another machine and keep in a dotfile
repository.

## Behaviour

An import profile is a configuration file (CORE-006) of the file type
`import-profile`, highest `schema_version` 1. Its keys (ADR 0032, decision 4):

| Key | Kind | Default | Meaning |
|---|---|---|---|
| `schema_version` | whole number, required | – | Version of the format, `1` |
| `video_destination` | path | the destination given on the command line | Folder to import videos into (IMPORT-013); left out, videos go to the destination given on the command line |
| `naming.template` | text | `{date:%Y}/{date:%Y-%m-%d}/{name}.{ext}` | Name template (IMPORT-001) |
| `naming.video_template` | text | `naming.template` | Name template for videos (IMPORT-018) |
| `naming.on_conflict` | text | `add_suffix` | Conflict policy (IMPORT-003): `add_suffix`, `skip` or `fail` |
| `job_codes` | list of text | empty | Job codes of the profile (IMPORT-006), the one to use first |
| `backup.destinations` | list of paths | empty | Backup destinations (IMPORT-007), in this order |
| `backup.video_destinations` | list of paths | the backup destinations | Backup destinations for videos (IMPORT-013), in this order |

`naming` and `backup` are mappings; either can be left out, and then each of its keys
takes its default.

**No destination.** The folder to import into is always given on the command line, so
that the meaning of the folders there never depends on a profile (ADR 0036, decision 8).
A top-level key `destination` is therefore an error at the key, in the form of
CORE-006, with a message that says where the folder goes instead:
`destination is not read from a profile; give the folder to import into on the command
line`. It replaces the message for an unknown key; `destination` inside `naming` or
`backup` is an ordinary unknown key. The format had no release with the key, so the
version stays 1.

**Paths.** A path is text that is absolute, or is `~`, or starts with `~/`. `~` means the
home folder that the caller passes when it reads the profile, so `~/Pictures` becomes
`<home>/Pictures`. Other text is an error at the value: `<key> must be an absolute path
or start with ~/`. A path is not checked against the file system here; the import does
that when it runs.

**Checks of the values.** Each is an error at the value, in the form of CORE-006:

- `naming.template`, `naming.video_template`: a template that IMPORT-001 rejects; the
  message is IMPORT-001's.
- `naming.on_conflict`: any other text: `expected add_suffix, skip or fail, found
  <value>`.
- `job_codes`: an entry that IMPORT-006 rejects, with IMPORT-006's message; and an entry
  that, after trimming, is equal to an earlier one: `repeated job code <job code>`.
  Entries are kept trimmed and in their order.
- `backup.destinations`: an entry that is not a valid path, and an entry equal to an
  earlier one: `repeated backup destination <path>`.
- `backup.video_destinations`: the same, with the message `repeated video backup
  destination <path>`. An empty list is an error at the value: `leave
  backup.video_destinations out to back up videos to the backup destinations`, because
  videos without a backup are not offered (IMPORT-013).

The keys for videos are optional, so every profile of `schema_version` 1 stays valid and
the version stays 1.

Reading a profile is pure: it depends only on the text, the file name for messages and
the home folder.

**JSON Schema.** The schema of the file type is published as
`schemas/import-profile-v1.json` with the `$id`
`https://polandy.github.io/framehive/schemas/import-profile-v1.json`, so a file that
starts with `# yaml-language-server: $schema=<that URL>` gets completion in an editor.
The schema's descriptions are the "Meaning" column above.

## Acceptance criteria

### IMPORT-011.1 – A minimal profile takes the defaults
- **Given** the text `schema_version: 1\n`
- **When** it is read as an import profile
- **Then** it has no video destination, the template `{date:%Y}/{date:%Y-%m-%d}/{name}.{ext}`,
  the policy `add_suffix`, no job codes and no backup destinations

### IMPORT-011.2 – A full profile
- **Given** the example profile of ADR 0032, decision 4, and the home folder `/home/ada`
- **When** it is read as an import profile
- **Then** its template and policy are
  those of the file, its job codes are `Smith wedding`, and its backup destination is
  `/run/media/ada/Backup/Pictures`

### IMPORT-011.3 – Relative paths are rejected
- **Given** the text `schema_version: 1\nvideo_destination: Videos\n`, and a text whose
  `backup.destinations` holds `../Backup`
- **When** each is read as an import profile
- **Then** the first is an error at line 2, column 20: `video_destination must be an
  absolute path or start with ~/`; the second is the same error for
  `backup.destinations` at that entry

### IMPORT-011.4 – `~` alone is the home folder
- **Given** the text `schema_version: 1\nvideo_destination: "~"\n` and the home folder
  `/home/ada`
- **When** it is read as an import profile
- **Then** its video destination is `/home/ada`

### IMPORT-011.5 – Invalid template
- **Given** a profile whose `naming.template` is `{date:%Y}/{unknown}.{ext}`
- **When** it is read as an import profile
- **Then** the error is at that value, with the message IMPORT-001 gives for the unknown
  token

### IMPORT-011.6 – Unknown conflict policy
- **Given** a profile whose `naming.on_conflict` is `overwrite`
- **When** it is read as an import profile
- **Then** the error is at that value: `expected add_suffix, skip or fail, found overwrite`

### IMPORT-011.7 – Job codes are checked and trimmed
- **Given** profiles whose `job_codes` are `[" Smith wedding "]`, `["a/b"]` and
  `[Smith, " Smith"]`
- **When** each is read as an import profile
- **Then** the first has the job code `Smith wedding`; the second is an error at `a/b`
  with IMPORT-006's message for `/`; the third is an error at the second entry:
  `repeated job code Smith`

### IMPORT-011.8 – Repeated backup destination
- **Given** a profile whose `backup.destinations` lists `/mnt/backup` twice
- **When** it is read as an import profile
- **Then** the error is at the second entry: `repeated backup destination /mnt/backup`

### IMPORT-011.9 – Unknown key inside a section
- **Given** a profile with `naming:\n  templte: "{name}.{ext}"\n`
- **When** it is read as an import profile
- **Then** the error is at `templte`: `unknown key templte; did you mean template?`

### IMPORT-011.12 – Video settings
- **Given** a profile with `video_destination: ~/Videos` and
  `backup.video_destinations: [/mnt/backup/Videos]`, a profile without either, and a
  profile with `backup.video_destinations: []`
- **When** each is read as an import profile with the home folder `/home/ada`
- **Then** the first has the video destination `/home/ada/Videos` and that video backup
  destination; the second has neither; the third is an error at the value with the
  message above

### IMPORT-011.13 – A destination in a profile
- **Given** the text `schema_version: 1\ndestination: ~/Pictures\n`, and a text with
  `destination: /a` inside `naming`
- **When** each is read as an import profile
- **Then** the first is an error at line 2, column 1: `destination is not read from a
  profile; give the folder to import into on the command line`; the second is an error
  at that key: `unknown key destination`

### IMPORT-011.10 – The published schema is the generated one
- **Given** the file type `import-profile`
- **When** its JSON Schema is generated
- **Then** it is identical to `schemas/import-profile-v1.json`, has the `$id` above, and
  lists `video_destination`, `naming`, `job_codes` and `backup`, and not `destination`

### IMPORT-011.11 – Examples validate
- **Given** every import profile shown in `docs/`, `specs/` and the output of
  `framehive config init` (CLI-010)
- **When** each is read as an import profile
- **Then** each is valid

## Out of scope

- Finding a profile by name, the configuration folder, `--profile` and `framehive config
  init`: CLI-010.
- Binding a profile to a device (N8, Q9), metadata (N16), actions after the import, the
  scanned folders (Q6) and the number of required locations (IMPORT-008): later versions
  of the format.
- Offering a profile's job codes in the guided import, and choosing a profile there:
  a later CLI-002 change.
- Saving a profile from Framehive (ADR 0011, rule 9).

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-05 | Initial draft (N8, N25, ADR 0032); accepted by the owner |
| 2026-11-05 | Implemented (reading and schema); the conflict policy is a choice of CORE-006 (CORE-006.20); value errors are at the start of the value |
| 2026-11-06 | `video_destination` and `backup.video_destinations` (IMPORT-013), optional in version 1; accepted by the owner |
| 2026-11-07 | `destination` removed: the folder to import into is given on the command line (ADR 0036, decision 8); a profile with it gets IMPORT-011.13; accepted by the owner |
| 2026-11-08 | `naming.video_template` (IMPORT-018), optional in version 1; owner decision #72 |
