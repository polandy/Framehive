# 0032 – Import profiles and the YAML parser

- Status: accepted
- Date: 2026-11-05
- Amends: ADR 0011, section "Library", which left the parser open

## Context

ADR 0011 makes YAML the format of every file a user edits, and leaves the parser to be
chosen against ADR 0009. The first such file is the import profile (N8): a named set of
import settings that `framehive import --profile <name>` uses (N18), so that a script
or a user does not repeat the destination, backup destinations, template and conflict
policy on every run. Configuration as YAML (N25) starts with it.

Three questions need an answer before code: which parser, where the parser and the
typed mapping live in the layers (architecture §3), and the format and location of a
profile file.

## Decision

### 1. `saphyr-parser` behind `adapter-yaml`

The YAML parser is `saphyr-parser` (pure Rust, MIT or Apache-2.0). It implements YAML
1.2, is tested against the official yaml-test-suite in its own repository, interprets
no tags, and reports a position for every event. It is the parser of the maintained
`saphyr` line that succeeded `yaml-rust`. libyaml was rejected: it is YAML 1.1 and
native code (ADR 0009).

Only the event parser is used, not `saphyr`'s document loader: our own code builds the
tree, so it keeps the position of every key and value and the style of every scalar
(plain or quoted), and it rejects what ADR 0011, rule 5, forbids while it builds.

### 2. A `config` service with a parser port

A new service crate `config` holds what every configuration file shares:

- the YAML tree (`YamlNode`): mappings with keys in file order, sequences and scalars,
  each with its line and column and, for a scalar, whether it was quoted;
- the port `YamlParser`, which turns text into that tree or a syntax error with a
  position;
- the strict reading of a tree into typed values: unknown and repeated keys, wrong
  types, a missing or unsupported `schema_version`, each as an error with file, line,
  column and, for an unknown key, a "did you mean …?" suggestion;
- a description of a file type's keys, from which the JSON Schema is generated.

`adapter-yaml` (port `config`) implements `YamlParser` with `saphyr-parser`; no other
crate sees one of its types. The typed structs of a file type live in their owner:
the import profile in the `import` module. Parsing text is pure, so the `import`
module reads a profile from text it is given; `apps/cli` reads the file and passes the
adapter.

### 3. One file per profile, named after the profile

Each import profile is one file `<config folder>/profiles/<name>.yaml`. The file name
without `.yaml` is the profile name, so there is no `name` key that could disagree with
it. A profile name has 1 to 64 characters from `a`–`z`, `0`–`9`, `-` and `_`: the
names stay the same on case-insensitive file systems and need no quoting in a shell.

The configuration folder follows architecture §6:

| Platform | Configuration folder |
|---|---|
| Linux | `$XDG_CONFIG_HOME/framehive`, or `$HOME/.config/framehive` when `XDG_CONFIG_HOME` is not set or not absolute |
| macOS | `$HOME/Library/Application Support/framehive` |

This differs from the state folder (ADR 0025, decision 1), which uses the XDG rule on
both platforms: state is not edited by hand, configuration is, and a macOS user looks
for it where other applications keep theirs.

### 4. The keys of a version-1 profile

A version-1 profile holds exactly the settings `framehive import` takes today. Every
key except `schema_version` is optional:

```yaml
# yaml-language-server: $schema=https://polandy.github.io/framehive/schemas/import-profile-v1.json
schema_version: 1

destination: ~/Pictures/Archive

naming:
  template: "{date:%Y}/{date:%Y-%m-%d}_{job}/{name}.{ext}"
  on_conflict: add_suffix          # add_suffix | skip | fail

job_codes:
  - Smith wedding

backup:
  destinations:
    - /run/media/ada/Backup/Pictures
```

Paths are absolute, or start with `~/`, which means the home folder. A profile lists
job codes rather than one, so the guided import can later offer them; `framehive
import` uses the first one unless `--job-code` is given. Binding a profile to a device,
metadata and actions after the import come with later versions of the format.

### 5. A given option replaces the profile's value

An option on the command line always wins over the profile, and it replaces the
profile's value rather than adding to it: one `--backup` replaces the profile's whole
list of backup destinations. One rule for every setting is easier to predict than a
rule per setting.

### 6. Our own JSON Schema generator

The JSON Schema of a file type is generated from the description in decision 2 by our
own code, not by `schemars`: one file type does not justify another dependency (ADR
0009), and the description is needed anyway for the strict reading. `cargo xtask
schemas` writes `schemas/import-profile-v1.json`; CI fails when the committed schema
differs from the generated one.

## Alternatives

- **`saphyr`'s document loader:** less code, but its tree has no positions for keys and
  drops the scalar style, so neither the error positions nor "quote this" (ADR 0011,
  rule 6) would be possible.
- **One file with all profiles:** fewer files, but one broken profile would break all,
  and a profile could not be shared by copying one file.
- **The XDG folder on macOS too, as for the state folder:** one rule for both
  platforms, but against architecture §6 and the macOS convention for settings.
- **Options add to the profile's lists:** convenient for one extra backup drive, but
  then a profile's backup destination could not be left out for one run.

## Consequences

- A new service crate `config` and a new adapter crate `adapter-yaml`, with
  `saphyr-parser` as a direct dependency in `dependencies.yaml`.
- The validation and its error messages are our own code, covered by CORE-006.
- Later file types (application settings, `library.yaml`, albums) reuse `config` and
  add only their typed structs and descriptions.
- Changing the keys of a profile means a new `schema_version` with a migration
  (ADR 0011, rule 7).
