# 0002 – Rust and Slint

- Status: accepted
- Date: 2026-10-17

## Context

Requirements: performant, appealing, mature; Linux and macOS as primary platforms,
Windows left open. Details and a comparison of toolkits: requirements §8.

## Decision

Implementation in Rust (stable, pinned). UI built with Slint. Slint code lives
exclusively in `*-ui` crates and in `ui-kit`; application logic lives in ViewModels in
plain Rust.

## Consequences

- The application logic is testable without Slint; a later toolkit switch would only
  affect the `*-ui` crates.
- The large thumbnail grid and the map have to be built ourselves (risks listed in
  requirements §8).
