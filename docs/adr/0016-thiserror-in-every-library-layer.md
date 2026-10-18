# 0016 – `thiserror` in every library layer

- Status: accepted
- Date: 2026-10-18

## Context

ADR 0009 allows only the standard library in `domain`, and requires an ADR for every
new dependency in `domain` or `kernel`. At the same time, the working rules
(`CLAUDE.md`, architecture §6) require `thiserror` error types in all libraries, with
every variant documented. The first crates of the repository scaffold (`domain`,
`kernel`) need error types, so the two rules must be reconciled.

## Decision

`thiserror` (version 2, `default-features = false`, feature `std`) is allowed in the
layers `domain`, `kernel`, `service`, `module` and `adapter`. It is listed in
`dependencies.yaml` with this ADR as its reference.

It is a compile-time-only dependency: the derive generates plain `Display` and `Error`
implementations, and the crate adds no runtime code, no I/O and no global state. It
therefore does not weaken the "no I/O in the domain" rule.

## Alternatives

- **Hand-written `Display` and `Error` implementations:** no dependency, but repetitive
  boilerplate in every crate, and it contradicts the project-wide rule.
- **`anyhow` in libraries:** loses typed errors; allowed only in apps and `xtask`
  (architecture §6).

## Consequences

- `domain` and `kernel` gain a proc-macro dependency tree (`thiserror-impl`, `syn`,
  `quote`, `proc-macro2`, `unicode-ident`). It is build-time only and widely shared.
- Further exceptions for `domain` (e.g. `serde` derive) still need their own ADR.
