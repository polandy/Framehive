# 0011 – Configuration in YAML

- Status: accepted
- Date: 2026-10-17

## Context

Users edit configuration by hand: application settings, import profiles, library
roots, albums and saved searches. These files are also shared between machines
(ADR 0003) and checked into dotfile repositories. The format must be expressive,
readable without documentation, and allow comments.

## Decision

All human-editable files use **YAML 1.2** (core schema). This covers application
settings, import profiles, `.framehive/library.yaml`, albums and saved searches under
`.framehive/`, and `dependencies.yaml`. Machine data (the index) stays in SQLite.

### Rules for expressive, understandable configuration

1. **Self-explanatory keys.** Use full English words in `snake_case`, such as
   `verify_copies_with_checksum` instead of `verify` or `vfy`. A key says what it does.
2. **Structure follows the user's mental model.** Nest keys the way the UI groups
   them, for example `destinations`, `backup`, `naming` and `metadata`.
3. **Explicit units and values.** Write sizes as `20 GiB` and durations as `15 min`.
   Use enums as words (`on_conflict: add_suffix`), never magic numbers or bare
   booleans for choices with more than two options.
4. **Sensible defaults.** Omitted keys take documented defaults, so a minimal file is
   valid. `framehive config init` writes a complete, commented example.
5. **Strict validation.** Unknown keys are errors, with a "did you mean …?"
   suggestion. Errors name the file, line, column and the expected value. YAML features
   that hurt readability are rejected: anchors and aliases, merge keys, custom tags,
   and multiple documents per file.
6. **No implicit typing surprises.** Parse with the YAML 1.2 core schema, so `no` is a
   string, not `false`. Strings that look like numbers or dates must be quoted, and the
   validator says so.
7. **Versioned schema.** Every file starts with `schema_version`. Older versions are
   migrated automatically, and the original is kept as a backup.
8. **Published schema.** A JSON Schema per file type is generated from the typed Rust
   config structs and stored in `schemas/`. Each file can reference it for editor
   completion (`# yaml-language-server: $schema=…`). CI fails if the generated schema
   drifts from the committed one. Every example file in docs, specs and tests must
   validate.
9. **Round-trip friendly.** When Framehive writes a file the user may have edited
   (for example a profile saved from the UI), it keeps comments and key order where the
   library allows. Otherwise it writes a canonical, commented form and keeps a backup.

### Example

```yaml
# yaml-language-server: $schema=https://framehive.example/schemas/profile-v1.json
schema_version: 1

name: Holiday
use_automatically_for: camera "Sony ILCE-7M4"

destinations:
  photos: ~/Pictures/Archive
  videos: ~/Videos/Archive
  folder_pattern: "{date:%Y}/{date:%Y-%m-%d}_{job}"
  file_name_pattern: "{date:%Y%m%d}-{time:%H%M}_{job}_{seq:4}"
  on_conflict: add_suffix          # add_suffix | skip | ask

backup:
  targets:
    - name: NAS
      path: /mnt/nas/photos
    - name: Samsung T7
      volume_marker: FRAMEHIVE_BACKUP
  card_is_safe_after_verified_copies: 2
  verify_copies_with_checksum: blake3

metadata:
  copyright: "© {year} Andy Pollari"
  keywords: [Holiday, Mountains]
  sidecar_naming: original_with_extension   # photo.ARW.xmp | photo.xmp: original_without_extension

after_import:
  eject_device: true
  run_command: [darktable, "{import_dir}"]   # argument list, no shell
```

### Library

`serde_yaml` is archived and unmaintained, so it does not meet ADR 0009. The parser is
chosen in phase 0 against the ADR 0009 criteria. It must support YAML 1.2, pass the
official yaml-test-suite and report positions. Candidates:

- `saphyr`: pure Rust, YAML 1.2, maintained successor line of `yaml-rust`.
- libyaml (the established C library behind PyYAML and Ruby's Psych), via a thin FFI.
  This one is YAML 1.1 and needs a native dependency.

The parser is wrapped in exactly one adapter crate (`adapter-yaml`). The typed mapping
and validation are our own code, so the parser stays replaceable.

## Alternatives

- **TOML:** simple and unambiguous, but deeply nested structures (backup targets,
  lists of objects) read poorly. The project owner chose YAML.
- **JSON:** no comments, poor to edit by hand.

## Consequences

- Config files are readable and commented, and editors can complete and validate
  them.
- YAML's pitfalls are contained by the strict subset, the core schema and the
  validator.
- More of our own code for validation and error messages. It is covered by spec and
  property tests.
