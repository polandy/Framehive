# CORE-006 – Reading YAML configuration files

- Status: implemented
- Module: core (`config` service, `adapter-yaml`)
- Origin: N25
- Depends on: –

## Purpose

Users write configuration files by hand (ADR 0011). A mistake must stop with a message
that says where it is and what was expected, and YAML's surprises (`no` meaning false,
aliases, tags) must not change what a file means.

## Behaviour

The `config` service reads the text of one configuration file into the values of its
file type, which describes its keys (name, type, required, default, one-line
description), such as the import profile (IMPORT-011). The caller passes the text and
the file's name for messages.

**YAML subset.** The text is YAML 1.2 with the core schema. These are errors at the
first character of the construct:

| Construct | Message |
|---|---|
| Anchor (`&name`) or alias (`*name`) | `anchors and aliases are not allowed` |
| Merge key (`<<`) | `merge keys are not allowed` |
| Tag (`!name`, `!!str`) | `tags are not allowed` |
| A second document (`---` after content, or a second `---`) | `only one document is allowed` |

One leading `---` and comments are allowed.

**Top level.** The document is a mapping. An empty text, a text with only comments, or
a top level that is a sequence or a scalar is an error at line 1, column 1:
`expected a mapping of keys`.

**`schema_version`.** Every file has the key `schema_version` with a plain integer. A
missing key, or one with no value, is an error at line 1, column 1. A value that is not
a positive integer is an error at the value. A version above the highest version the
file type knows is an error at the value: `schema_version <n> is newer than this
Framehive supports (<highest>); update Framehive`.

**Keys.** A key is a plain or quoted string. A key that the file type does not know is
an error at the key: `unknown key <key>`, followed by `; did you mean <known>?` when a
known key at the same level differs from it by at most 2 single-character insertions,
deletions or substitutions (the nearest one; on a tie the first in the description's
order). A key that appears twice in the same mapping is an error at the second one:
`repeated key <key>`. A missing required key is an error at the mapping that should hold
it: `missing key <key>`.

**Values.** A key with no value (YAML null, written as nothing, `~` or `null`) is an
error at the key: `<key> has no value; leave the key out to use the default`. A value of
the wrong kind is an error at the value: `expected <expected>, found <found>`, each
named as in the table below. A null list entry is an error at it: `<key> has an entry
with no value`.

| Expected | Accepted |
|---|---|
| text | a quoted scalar, or a plain scalar that the core schema reads as a string |
| a whole number | a plain scalar the core schema reads as an integer |
| true or false | a plain `true`, `True`, `TRUE`, `false`, `False` or `FALSE` |
| a list | a block or flow sequence |
| a mapping | a block or flow mapping |

So `no`, `yes`, `on` and `off` are text. A plain scalar that is an integer, a float, a
boolean, or starts with a date `YYYY-MM-DD`, where text is expected, is an error at the
value: `write "<value>" in quotes to use it as text`. A choice is text from a fixed
list; other text is an error at the value: `expected <a>, <b> or <c>, found <value>`.

**Errors.** Reading stops at the first error: the parser's (syntax, YAML subset), then
`schema_version`, which decides the known keys, then keys and values in file order,
then each mapping's missing keys. Its message is `<file>:<line>:<column>: <message>`,
line and column from 1, the column in characters. A syntax error has the
YAML parser's position and description.

**JSON Schema.** For each file type, the service generates a JSON Schema (draft
2020-12) from the description: an object per mapping, with every key as a property with
its type and description, `additionalProperties: false`, the required keys in
`required`, a choice's texts in `enum`, and `schema_version` as a constant of the
highest version. The generated text is the same on every run.

## Acceptance criteria

### CORE-006.1 – A minimal file takes the defaults
- **Given** a file type with an optional text key `destination` and the text
  `schema_version: 1\n`
- **When** it is read
- **Then** it is valid, and `destination` has its default

### CORE-006.2 – Unknown key with a suggestion
- **Given** a file type with the key `destination`, and the text
  `schema_version: 1\ndestinaton: /a\n` in the file `holiday.yaml`
- **When** it is read
- **Then** the error is `holiday.yaml:2:1: unknown key destinaton; did you mean destination?`

### CORE-006.3 – Unknown key without a suggestion
- **Given** the file type of CORE-006.2 and the text `schema_version: 1\ncolour: red\n`
- **When** it is read
- **Then** the error is at line 2, column 1: `unknown key colour`, with no suggestion

### CORE-006.4 – Repeated key
- **Given** the text `schema_version: 1\ndestination: /a\ndestination: /b\n`
- **When** it is read
- **Then** the error is at line 3, column 1: `repeated key destination`

### CORE-006.5 – Anchors and aliases
- **Given** a text with an anchor `&target /a` and, in another key, the alias `*target`
- **When** it is read
- **Then** the error is at the anchor: `anchors and aliases are not allowed`

### CORE-006.6 – Merge keys
- **Given** a text with a mapping that has the key `<<`
- **When** it is read
- **Then** the error is at that key: `merge keys are not allowed`

### CORE-006.7 – Tags
- **Given** the text `schema_version: 1\ndestination: !!str /a\n`
- **When** it is read
- **Then** the error is at line 2, column 14: `tags are not allowed`

### CORE-006.8 – One document only
- **Given** the texts `schema_version: 1\n---\nschema_version: 1\n` and
  `---\nschema_version: 1\n`
- **When** each is read
- **Then** the first is an error at line 2, column 1: `only one document is allowed`;
  the second is valid

### CORE-006.9 – `no` is text
- **Given** the text `schema_version: 1\ndestination: no\n`
- **When** it is read
- **Then** it is valid, and `destination` is the text `no`

### CORE-006.10 – Text that looks like a number or a date must be quoted
- **Given** a file type with a list of text `job_codes`, and the texts with
  `job_codes: [2026]`, `job_codes: [true]` and `job_codes: [2026-11-05 Smith]`
- **When** each is read
- **Then** each is an error at the value, for example `write "2026" in quotes to use it
  as text`; with `job_codes: ["2026"]` the job code is the text `2026`

### CORE-006.11 – Wrong kind of value
- **Given** the text `schema_version: 1\ndestination: [/a, /b]\n`
- **When** it is read
- **Then** the error is at line 2, column 14: `expected text, found a list`

### CORE-006.12 – A key without a value
- **Given** the text `schema_version: 1\ndestination:\n`
- **When** it is read
- **Then** the error is at line 2, column 1: `destination has no value; leave the key out
  to use the default`

### CORE-006.13 – Missing or invalid `schema_version`
- **Given** the texts `destination: /a\n`, `schema_version: "1"\n`,
  `schema_version: 0\n` and `schema_version:\n`
- **When** each is read
- **Then** the errors are `1:1: missing key schema_version`, `1:17: expected a whole
  number, found text`, `1:17: schema_version must be a positive whole number` and
  `1:1: schema_version has no value; it must be a positive whole number`

### CORE-006.14 – A newer `schema_version`
- **Given** a file type whose highest version is 1, and the text `schema_version: 2\n`
- **When** it is read
- **Then** the error is at line 1, column 17: `schema_version 2 is newer than this
  Framehive supports (1); update Framehive`

### CORE-006.15 – The top level is not a mapping
- **Given** the empty text, the text `# only a comment\n` and the text `- a\n`
- **When** each is read
- **Then** each is an error at line 1, column 1: `expected a mapping of keys`

### CORE-006.16 – YAML syntax error
- **Given** the text `schema_version: 1\ndestination: "/a\n` (an unclosed quote) in
  the file `holiday.yaml`
- **When** it is read
- **Then** the error starts with `holiday.yaml:` and the parser's line and column, and
  nothing is read

### CORE-006.17 – Missing required key
- **Given** a file type with a mapping `backup` whose key `destinations` is required,
  and the text `schema_version: 1\nbackup: {}\n`
- **When** it is read
- **Then** the error is at line 2, column 9: `missing key destinations`

### CORE-006.18 – The JSON Schema follows the description
- **Given** a file type with the optional text key `destination` and the description
  `Folder to import into`
- **When** its JSON Schema is generated twice
- **Then** both texts are identical, the schema has `additionalProperties: false`,
  `schema_version` is required with the constant 1, and `destination` is a property of
  type `string` with that description

### CORE-006.19 – Columns count characters
- **Given** the file type of CORE-006.10 and the text
  `schema_version: 1\njob_codes: ["Hochzeit Müller", 2026]\n`
- **When** it is read
- **Then** the error is at line 2, column 32, counting `ü` as one character

### CORE-006.20 – A choice accepts only its texts
- **Given** a choice `on_conflict` of `add_suffix`, `skip` and `fail`, and the text
  `schema_version: 1\non_conflict: overwrite\n`
- **When** it is read, and the JSON Schema is generated
- **Then** the error is at line 2, column 14: `expected add_suffix, skip or fail, found
  overwrite`; the schema has `"enum": ["add_suffix", "skip", "fail"]`

## Out of scope

- Finding and reading the file, and the configuration folder: CLI-010.
- Migrating older versions, writing a file the user may have edited, and units such as
  `20 GiB` (ADR 0011, rules 7, 9 and 3): with the first file type, file and key that
  needs one.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-05 | Initial draft with the import profiles (N8, N25, ADR 0032); accepted by the owner |
| 2026-11-05 | Implemented; error order and null list entries made explicit |
| 2026-11-05 | Choices (CORE-006.20) |
| 2026-11-07 | `schema_version:` without a value (CORE-006.13) |
