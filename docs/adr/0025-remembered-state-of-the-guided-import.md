# 0025 – Remembered state of the guided import

- Status: accepted
- Date: 2026-11-01

## Context

The guided import (ADR 0024, CLI-002, CLI-003) asks once for the destination and the name
template, and asks for a job code at each import whose template has `{job}`. It must
remember these answers, so that the next card needs only Enter, and it offers the job
codes used before (IMPORT-006).

ADR 0011 makes every human-editable file YAML with a schema, strict validation and a
`schema_version`. The settings and import profiles (N8) are such files and come later,
with a YAML parser. The answers of the guided import are not settings a person writes:
Framehive writes them, and the dialog is the way to change them. Pulling the YAML
configuration forward for three values would add a dependency (ADR 0009) and decide the
settings layout before profiles are designed.

## Decision

### 1. A state file, written by Framehive

The guided import keeps its answers in the state file
`$XDG_STATE_HOME/framehive/guided-import.txt`, or
`$HOME/.local/state/framehive/guided-import.txt` when `XDG_STATE_HOME` is not set or
not absolute. The same rule applies on macOS, as with other command-line programs, so
that one rule holds on both platforms. The folder is created when the file is first
written.

It is state, not configuration: it is not documented as something to edit, it has no
JSON Schema, and `framehive config init` does not write it. When profiles exist, they
take precedence, and the remembered answers stay a convenience.

### 2. Plain text, one value per line

```
# Framehive remembers the answers of the guided import here.
format_version=1
destination=/home/ada/Pictures
template={date:%Y}/{date:%Y-%m-%d}_{job}/{name}.{ext}
job_code=Smith wedding
job_code=Harbour
```

- The first line that is not empty and not a comment is `format_version=1`.
- Every other line is `<key>=<value>`, split at the first `=`. Keys are full words
  (ADR 0011, rule 1). Lines starting with `#` and empty lines are ignored.
- `destination` and `template` appear at most once; `job_code` repeats, most recently
  used first.
- A value cannot contain a line break. A destination whose path is not valid UTF-8 or
  contains a line break is not remembered and is asked for again next time.

### 3. A broken file never stops an import

When the file is missing, the dialog starts as on the first run. When it cannot be
read, has another `format_version`, or has a line it does not understand, the dialog
warns on standard error, ignores the file and starts as on the first run; the next
successful write replaces the file. A value that is invalid, such as a template that
does not parse or a job code that breaks IMPORT-006, is dropped with a warning, and the
rest is kept.

### 4. Written whole, never half

After each answer that changes it, the dialog writes the whole file to
`guided-import.txt.part`, syncs it, removes the old file and renames the new one into
place with the `FileSystem` port's `rename_no_replace`. A crash never leaves half a
file; a crash between removing and renaming loses the remembered answers, and the next
start is a first run. The port gets no replacing rename for this. A failed write is a
warning; the import goes on.

### 5. Where the code lives

Reading and writing the format are pure functions in `apps/cli/src/guided/`, and the
file is read and written through the `FileSystem` port. The rules of the remembered
job-code list (order, length, duplicates) belong to the import module (IMPORT-006).

## Alternatives

- **YAML now:** consistent with ADR 0011, but needs the YAML parser and its adapter
  before the settings are designed, for three values only Framehive writes.
- **The settings file of the later configuration:** would mix state that changes on
  every import with settings a person maintains, and would make Framehive rewrite a
  file the person may have commented.
- **The platform's application data folder on macOS** (`~/Library/Application
  Support`): native for apps, unusual for command-line programs, and a second rule.

## Consequences

- Deleting the file resets the guided import to its first run.
- The format is versioned; a later version, or the YAML settings, can read version 1
  and migrate it.
- Two guided imports that run at the same time write the file last-writer-wins; the
  remembered answers of the other are lost, nothing else.
