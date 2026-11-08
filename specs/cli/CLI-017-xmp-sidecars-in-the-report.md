# CLI-017 – XMP sidecars in the report

- Status: implemented
- Module: cli (`apps/cli`)
- Origin: N16, R9, N18
- Depends on: CLI-001, CLI-003, CLI-005, IMPORT-019, IMPORT-020

## Purpose

A photographer sees which sidecars `framehive import` and the guided import wrote, which
a dry run would write, and which values a sidecar from the card already had, so a value
the import did not write is not a surprise later.

## Behaviour

**Report of `framehive import`.** After a file's line (CLI-001) and before its backup
lines (CLI-005), each sidecar the file got has an indented line:

| Line | When |
|---|---|
| `  wrote sidecar  <target path>` | The import wrote the values into a new sidecar (IMPORT-019) |
| `  wrote sidecar  <source path> -> <target path>` | A source sidecar came with the file (IMPORT-020) |

Paths are relative to the source and to the file's destination, as in CLI-001. A source
sidecar counts as written when it was written and verified at the import destination or
at a backup destination. A failed file has no sidecar line; its line names the sidecar
that failed. A sidecar already at the target of a file found already imported is not
written and has no line. With `--dry-run`, `wrote sidecar` reads `would write sidecar`.

Then each value of the carried-along sidecar that differs from the import's has a line
(IMPORT-020.7):

| Line | When |
|---|---|
| `  replaced job code  <old> -> <new>` | The import's job code replaced the sidecar's |
| `  kept <field>  <value> (not <import value>)` | The sidecar's creator or copyright was kept |

`<field>` is `creator` or `copyright`; several creators are joined with `; `.

After the summary line of CLI-001, a run that wrote at least one sidecar prints
`<n> sidecars written`, where `<n>` counts the sidecar lines. A dry run prints no such
line.

**Guided import.** The plan summary (CLI-003) shows the values the import writes, below
the `named` and `job code` lines, each only when it is set:

```
  creator   <creator>; <creator>
  copyright <copyright, {year} as written in the configuration>
  keywords  <keyword>, <keyword>
```

The listing (`l`) prints the dry-run lines above. After the import, the report of
CLI-003 also prints the line of each file with a replaced or kept value, followed by
those value lines, and after the summary line the line `<n> sidecars written` above.

## Acceptance criteria

### CLI-017.1 – A written sidecar is reported
- **Given** `config.yaml` with the creator `Andy Pollari` and a card with
  `DCIM/100/DSCF0001.RAF`
- **When** `framehive import` runs
- **Then** the line of the file is followed by `  wrote sidecar  <target>.xmp`, and the
  summary line by `1 sidecars written`

### CLI-017.2 – A dry run lists the sidecars it would write
- **Given** the card and `config.yaml` of CLI-017.1
- **When** `framehive import --dry-run` runs
- **Then** the line of the file is followed by `  would write sidecar  <target>.xmp`, no
  sidecar exists, and no `sidecars written` line is printed

### CLI-017.3 – A carried-along sidecar and its replaced and kept values
- **Given** `DSCF0001.RAF` with `DSCF0001.RAF.xmp` holding the job code `Porto` and the
  creator `Camera Owner`, the job code `Lisbon` and the creator `Andy Pollari`
- **When** the file is imported
- **Then** its line is followed by
  `  wrote sidecar  DCIM/100/DSCF0001.RAF.xmp -> <target>.xmp`,
  `  replaced job code  Porto -> Lisbon` and
  `  kept creator  Camera Owner (not Andy Pollari)`

### CLI-017.4 – Without values, nothing changes
- **Given** no `metadata` values, no job code and no source sidecar
- **When** `framehive import` runs
- **Then** the report has no sidecar line and no `sidecars written` line

### CLI-017.5 – The guided plan summary shows the values
- **Given** `config.yaml` with the creators `Andy Pollari` and `Ada`, the copyright
  `© {year} Andy Pollari` and the keywords `Holiday/Beach` and `Family`
- **When** the plan summary is shown
- **Then** it has `  creator   Andy Pollari; Ada`, `  copyright © {year} Andy Pollari`
  and `  keywords  Holiday/Beach, Family`

### CLI-017.6 – The guided report shows kept values and the sidecar count
- **Given** a card whose `DSCF0001.RAF.xmp` holds the creator `Camera Owner`, and
  `config.yaml` with the creator `Andy Pollari`
- **When** the guided import runs
- **Then** the report has the file's `copied` line followed by
  `  kept creator  Camera Owner (not Andy Pollari)`, and `1 sidecars written` after the
  summary line

## Out of scope

- Writing the sidecars and merging the values: IMPORT-019, IMPORT-020.
- `framehive config init` and the `metadata` keys of the example: CLI-010.
- Showing the values of each sidecar, and keywords added by the union (IMPORT-020).

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-08 | Created and implemented under the owner's XMP decision D5, which leaves the CLI output of IMPORT-019 and IMPORT-020 to the CLI specs |
