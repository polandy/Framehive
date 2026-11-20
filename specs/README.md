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
├── cli/                  # command line (apps/cli)
├── devices/              # devices service (device detection, eject)
├── previews/             # previews service (RAW decode, previews)
├── map/  people/  …     # other modules
```

File name: `<ID>-<short-name>.md`, e.g. `IMPORT-002-sequence-numbers.md`.

## IDs

- Prefix per module or area: `CORE`, `IMPORT`, `LIBRARY`, `MAP`, `PEOPLE`,
  `SIMILARITY`, `MIGRATE`, `IMMICH`, `CLI`, `DEVICES`,
  `PREVIEWS`.
- Sequential number, three digits, never reused.
- Acceptance criteria: `<ID>.<n>`, e.g. `IMPORT-002.3`. Numbers are never reassigned;
  removed criteria stay listed, with `(removed)` at the end of the heading, and need
  no test.

## Status

| Status | Meaning |
|---|---|
| `draft` | In progress, not yet binding |
| `accepted` | Binding, not yet implemented, not reachable in the code |
| `partial` | Binding, partly implemented; the criteria in `- Pending:` are not reachable in the code yet, every other one has at least one test |
| `implemented` | Behaviour is implemented; every criterion has at least one test |
| `removed` | Feature removed; justification in the "History" section |

## Rules

1. Describe behaviour that can be observed from the outside: via the UI, the CLI,
   files, or a public service API. Implementation details belong in the code and the
   crate README.
2. Every criterion is unambiguously verifiable, as Given / When / Then.
3. Use terms from the glossary (`docs/dev/architecture/10-glossary.md`).
4. Edge cases and error behaviour are their own criteria.
5. One feature per file; prefer several small specifications over one large one.
6. Tests reference the criterion they verify with `/// spec: <ID>.<n>`, written on the
   `#[test]` function itself.
7. Every `CLI-*` criterion has a test in `apps/cli/tests/` that runs the real binary.
   Until it does, or where the binary cannot reach it, the criterion is listed in
   `- E2e-owed:` (ADR 0010, amendment 2026-11-20).
8. If the specification changes, tests and code change in the same commit, and vice
   versa.

## Template

```markdown
# <ID> – <Title>

- Status: draft | accepted | partial | implemented | removed
- Pending: <only for `partial`: the criterion numbers not built yet, e.g. 1, 2, 8>
- E2e-owed: <only for `CLI-*`: criteria still without a test in apps/cli/tests/, e.g. 2, 4–6>
- Module: <module>
- Origin: <requirement IDs from docs/dev/requirements/, e.g. R4, R5>
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
