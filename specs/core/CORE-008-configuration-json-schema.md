# CORE-008 – JSON Schema of configuration files

- Status: implemented
- Module: core (`config` service)
- Origin: N25
- Depends on: CORE-006

## Purpose

Editors check and complete a hand-written configuration file against a JSON Schema
(ADR 0011). The schema must say exactly what reading the file accepts (CORE-006), and
it must not change between runs, so that `cargo xtask check-schemas` can compare it
with the committed file.

## Behaviour

For each file type, the `config` service generates a JSON Schema (draft 2020-12) from
the description of its keys (CORE-006): an object per mapping, with every key as a
property with its type and description, `additionalProperties: false`, the required
keys in `required`, a choice's texts in `enum`, and `schema_version` as a constant of
the highest version. The generated text is the same on every run.

## Acceptance criteria

### CORE-008.1 – The JSON Schema follows the description
- **Given** a file type with the optional text key `destination` and the description
  `Folder to import into`
- **When** its JSON Schema is generated twice
- **Then** both texts are identical, the schema has `additionalProperties: false`,
  `schema_version` is required with the constant 1, and `destination` is a property of
  type `string` with that description

### CORE-008.2 – A choice lists its texts
- **Given** a choice `on_conflict` of `add_suffix`, `skip` and `fail`
- **When** the JSON Schema is generated
- **Then** the property `on_conflict` has the type `string` and
  `"enum": ["add_suffix", "skip", "fail"]`

## Out of scope

- Writing the schema files to `schemas/` and checking them: IMPORT-011,
  `cargo xtask schemas` and `cargo xtask check-schemas`.

## Open questions

–

## History

| Date | Change |
|---|---|
| 2026-11-08 | Split from CORE-006 (was CORE-006.18 and the schema part of CORE-006.20) so that CORE-006 stays below 10 KB |
