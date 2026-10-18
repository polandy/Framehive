# 1. Principles

Part of the [architecture](README.md).

1. **Modules are isolated.** A module (import, library, map, people, …) knows no
   other module. Collaboration happens only through the kernel: events, commands and
   shared services with stable interfaces.
2. **Business logic is free of I/O.** Business logic reaches the file system, devices,
   database, clock, randomness and network only through ports (traits). The
   implementations (adapters) live in their own crates ("functional core, imperative
   shell").
3. **Standards before custom solutions.** Where a standard exists, we implement it
   exactly and test against it (Section 8). Custom formats only where no standard
   exists, and then openly documented.
4. **Rules are checked by machine, not just written down.** Since presumably only AI
   agents will write the code, there is no human review as a safety net. Every
   architecture rule has an automatic check in CI (Section 7).
5. **Readability for humans is a goal, not a by-product.** Code is written so that a
   human can follow it without prior knowledge: unambiguous names from the glossary
   (Section 10), small units, comments explain the *why*.
6. **Determinism.** Same input, same result: in builds, in tests, in import plans. No
   flakiness, no retries in CI.
7. **Minimal dependencies.** Standard library or our own, well-tested code first;
   libraries only where they clearly make sense, and only from established products.
   Every external library is encapsulated in exactly one adapter (ADR 0009).
8. **Specification first.** Every feature is specified in `specs/`. Specification,
   tests and implementation always change together, in both directions (ADR 0010).
9. **Extensible via defined extension points.** New modules, device types, metadata
   formats or export targets are added without touching existing modules.
