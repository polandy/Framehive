# CORE-006 – Reading YAML configuration files

- Status: accepted
- Module: core (`config` service, `adapter-yaml`)
- Origin: N25
- Depends on: –

## Purpose

Users write Framehive's configuration files by hand (ADR 0011). A mistake in such a file
must stop with a message that says where it is and what was expected, rather than
being guessed around, and YAML's surprises (`no` meaning false, aliases, tags) must not
change what a file means.

## Behaviour

The `config` service reads the text of one configuration file into the typed values of
its file type. A file type (for example the import profile, IMPORT-011) describes its
keys: name, type, whether it is required, its default and a one-line description. The
caller passes the text and the file's name for messages.

**YAML subset.** The text is YAML 1.2 with the core schema. The following are errors,
with the position of the first character of the offending construct:

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
missing key is an error at line 1, column 1. A value that is not a positive integer is
an error at the value. A version above the highest version the file type knows is an
error at the value: `schema_version <n> is newer than this Framehive supports
(<highest>); update Framehive`. Migrating older versions (ADR 0011, rule 7) begins
with the first file type that has a version 2.

**Keys.** A key is a plain or quoted string. A key that the file type does not know is
an error at the key: `unknown key <key>`, followed by `; did you mean <known>?` when a
known key at the same level differs from it by at most 2 single-character insertions,
deletions or substitutions (the nearest one; on a tie the first in the description's
order). A key that appears twice in the same mapping is an error at the second one:
`repeated key <key>`. A missing required key is an error at the mapping that should hold
it: `missing key <key>`.

**Values.** A key with no value (YAML null, written as nothing, `~` or `null`) is an
error at the key: `<key> has no value; leave the key out to use the default`. A value of
the wrong kind is an error at the value that names what was expected: `expected
<expected>, found <found>`, where each is one of `text`, `a whole number`, `true or
false`, `a list` and `a mapping`.

| Expected | Accepted |
|---|---|
| text | a quoted scalar, or a plain scalar that the core schema reads as a string |
| a whole number | a plain scalar the core schema reads as an integer |
| true or false | a plain `true`, `True`, `TRUE`, `false`, `False` or `FALSE` |
| a list | a block or flow sequence |
| a mapping | a block or flow mapping |

So `no`, `yes`, `on` and `off` are text. Text that the core schema would read as
something else, or that looks like a date, is an error when it is written plain: a
plain scalar that is an integer, a float, a boolean, or starts with a date
`YYYY-MM-DD`, in a place that expects text, is an error at the value: `write "<value>"
in quotes to use it as text`.

**Errors.** Reading stops at the first error in file order. Its message is
`<file>:<line>:<column>: <message>`, with line and column counted from 1, the column in
characters. A syntax error of the YAML parser is reported in the same form, with the
parser's position and description.

**JSON Schema.** For each file type, the service generates a JSON Schema (draft
2020-12) from the description: an object per mapping, with every key as a property with
its type and description, `additionalProperties: false`, the required keys in
`required`, and `schema_version` as a constant of the highest version. The generated
text is the same on every run.

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
- **Then** the error is at line 2, column 1, is `unknown key colour`, and has no
  suggestion

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
- **Given** the text `schema_version: 1\n---\nschema_version: 1\n`, and the text
  `---\nschema_version: 1\n`
- **When** each is read
- **Then** the first is an error at line 2, column 1: `only one document is allowed`;
  the second is valid

### CORE-006.9 – `no` is text
- **Given** a file type with a text key `destination`, and the text
  `schema_version: 1\ndestination: no\n`
- **When** it is read
- **Then** it is valid, and `destination` is the text `no`

### CORE-006.10 – Text that looks like a number or a date must be quoted
- **Given** a file type with a list of text `job_codes`, and the texts with
  `job_codes: [2026]`, `job_codes: [true]` and `job_codes: [2026-11-05 Smith]`
- **When** each is read
- **Then** each is an error at the value, for example `write "2026" in quotes to use it
  as text`; with `job_codes: ["2026"]` the job code is the text `2026`

### CORE-006.11 – Wrong kind of value
- **Given** a file type with a text key `destination`, and the text
  `schema_version: 1\ndestination: [/a, /b]\n`
- **When** it is read
- **Then** the error is at line 2, column 14: `expected text, found a list`

### CORE-006.12 – A key without a value
- **Given** the text `schema_version: 1\ndestination:\n`
- **When** it is read
- **Then** the error is at line 2, column 1: `destination has no value; leave the key out
  to use the default`

### CORE-006.13 – Missing or invalid `schema_version`
- **Given** the texts `destination: /a\n`, `schema_version: "1"\n` and
  `schema_version: 0\n`
- **When** each is read
- **Then** the first is an error at line 1, column 1: `missing key schema_version`; the
  second is `expected a whole number, found text` at line 1, column 17; the third is an
  error at line 1, column 17 that says the version must be a positive whole number

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
- **Then** the error starts with `holiday.yaml:` followed by the line and column the
  parser reports, and nothing is read

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

## Out of scope

- The keys of a particular file type: the import profile is IMPORT-011.
- Finding and reading the file, and the configuration folder: CLI-010.
- Migrating older versions (ADR 0011, rule 7): with the first file type that has a
  version 2.
- Writing a file the user may have edited (ADR 0011, rule 9): with the first file
  Framehive writes, such as a profile saved from the UI.
- Units such as `20 GiB` or `15 min` (ADR 0011, rule 3): with the first key that needs
  one.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-05 | Initial draft with the import profiles (N8, N25, ADR 0032); accepted by the owner |
