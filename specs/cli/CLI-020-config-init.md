# CLI-020 – `framehive config init`

- Status: implemented
- Module: cli (`apps/cli`)
- Origin: N8, N25
- Depends on: CLI-010, CLI-019, IMPORT-011, IMPORT-019

## Purpose

`framehive config init` writes a commented example import profile to start from, so
nobody has to learn the keys of IMPORT-011 from the schema alone.

## Behaviour

```
framehive config init
```

Writes the example profile `<configuration folder>/profiles/example.yaml`, with the
configuration folder of CLI-010, creating missing folders, prints `wrote <path>` and
exits with 0. The example has every key of IMPORT-011 and IMPORT-019, each with a
comment that explains it and names its default, and the `yaml-language-server` line of
IMPORT-011. When the file exists, nothing is written, standard error gets `error: <path>
exists already; nothing was written`, and the exit code is 1. A folder or the file that
cannot be written gives exit code 1 and `error: the folder <path> cannot be created:
<reason>` or `error: <path> cannot be written: <reason>`, the reason in the words of
CLI-019.

## Acceptance criteria

### CLI-020.1 – `config init` writes the example
- **Given** an empty configuration folder
- **When** `framehive config init` runs
- **Then** `<folder>/profiles/example.yaml` exists, standard output has `wrote <path>`,
  the file is a valid import profile with the defaults of IMPORT-011 and IMPORT-019,
  every key of both appears in it, and the exit code is 0

### CLI-020.2 – `config init` keeps an existing example
- **Given** a `profiles/example.yaml` with the text `# mine`
- **When** `framehive config init` runs
- **Then** the file still holds `# mine`, standard error has `error: <path> exists
  already; nothing was written`, and the exit code is 1

## Out of scope

- Profile keys and checks: IMPORT-011. The configuration folder: CLI-010.
- `framehive config` subcommands other than `init`.

## History

| Date | Change |
|---|---|
| 2026-11-19 | Moved from CLI-010.15 and CLI-010.16 without a change in behaviour (review E26) |
