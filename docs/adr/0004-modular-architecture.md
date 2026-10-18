# 0004 – Modular architecture with a kernel and static registration

- Status: proposed
- Date: 2026-10-17

## Context

Framehive consists of domain modules (import, library, map, people, …). New modules
should be addable without changes to existing modules. The modules should be clearly
separated from each other and have clear responsibilities.

## Decision

- Each module consists of a domain crate (`<module>`) and a UI crate (`<module>-ui`)
  and implements the `kernel::Module` contract.
- Modules never depend on each other. They communicate via the kernel's typed events
  and commands and use shared services (`catalog`, `metadata`, `jobs`, …) through
  their public API.
- Each module owns its own tables (prefixed), its own migrations and its own settings.
- Registration is static in the apps. No dynamically loaded plugins.

## Alternatives

- *One large crate with Rust modules*: simpler at first, but boundaries cannot be
  enforced.
- *Dynamic plugins (`dlopen`)*: no stable Rust ABI, hard to test.
- *WebAssembly plugins*: possible for later third-party plugins, own ADR.

## Consequences

- Boundaries are enforced by the compiler and by `cargo xtask check-architecture`.
- The kernel contract becomes the critical interface that must be kept stable
  (`cargo public-api` snapshot).
- More crates, somewhat longer builds; manageable through workspace caching.
