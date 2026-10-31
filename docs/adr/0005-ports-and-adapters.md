# 0005 – Ports & adapters, domain logic without I/O

- Status: accepted
- Date: 2026-10-17

## Context

The best testability and determinism require that domain logic can be tested without
real devices, file systems, clocks or databases.

## Decision

- Layers: `domain` → `kernel` → `service` → `module`; adapters implement ports; only
  the apps wire things together (rules in requirements §3).
- I/O, time (`Clock`), IDs (`IdGenerator`) and randomness (`Rng`) are only accessed
  through ports.
- Every port has a conformance test suite that all implementations must pass, as well
  as an in-memory or fake implementation for tests.

## Consequences

- Domain logic is fast and fully unit-testable.
- More indirection; ports are only introduced where I/O or non-determinism is
  involved.
