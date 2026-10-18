# Specifications

Every feature of Framehive is specified here (ADR 0010). The specification is the
binding description of observable behaviour. Specification, tests and
implementation always change together.

## Layout

```
specs/
├── README.md            # this file: rules and template
├── core/                # kernel, services (catalog, metadata, jobs, volumes, …)
├── import/              # import module
├── library/              # library module
├── map/  people/  …     # other modules
```

File name: `<ID>-<short-name>.md`, e.g. `IMPORT-002-sequence-numbers.md`.

## IDs

- Prefix per module or area: `CORE`, `IMPORT`, `LIBRARY`, `MAP`, `PEOPLE`,
  `SIMILARITY`, `MIGRATE`, `IMMICH`, `CLI`.
- Sequential number, three digits, never reused.
- Acceptance criteria: `<ID>.<n>`, e.g. `IMPORT-002.3`. Numbers are never reassigned;
  removed criteria stay listed, marked "(removed)".

## Status

| Status | Meaning |
|---|---|
| `draft` | In progress, not yet binding |
| `accepted` | Binding, not yet implemented, not reachable in the code |
| `implemented` | Behaviour is implemented; every criterion has at least one test |
| `removed` | Feature removed; justification in the "History" section |

## Rules

1. Describe behaviour that can be observed from the outside: via the UI, the CLI,
   files, or a public service API. Implementation details belong in the code and the
   crate README.
2. Every criterion is unambiguously verifiable, as Given / When / Then.
3. Use terms from the glossary (`docs/architecture/10-glossary.md`).
4. Edge cases and error behaviour are their own criteria.
5. One feature per file; prefer several small specifications over one large one.
6. Tests reference the criterion they verify with `/// spec: <ID>.<n>`.
7. If the specification changes, tests and code change in the same commit, and vice
   versa.

## Template

```markdown
# <ID> – <Title>

- Status: draft | accepted | implemented | removed
- Module: <module>
- Origin: <requirement IDs from docs/requirements/, e.g. R4, R5>
- Depends on: <other spec IDs or "–">

## Purpose

<One to three sentences: what problem the feature solves, for whom.>

## Behaviour

<Description in prose: flows, rules, terms. Tables where helpful.>

## Acceptance criteria

### <ID>.1 – <short title>
- **Given** …
- **When** …
- **Then** …

### <ID>.2 – …

## Out of scope

<What explicitly does not belong here, with a reference to where it is specified
instead.>

## Open questions

<Only in `draft` status. Must be empty before `accepted`.>

## History

| Date | Change |
|---|---|
```
