# 0008 – Development by AI agents: enforce rules mechanically

- Status: proposed
- Date: 2026-10-17

## Context

No human is expected to write code in this project. The code must still be
understandable and traceable for humans, and quality must not depend on human review.

## Decision

- Every architecture and style rule has an automated check (`docs/architecture/`
  section 7). Whatever is not checked counts as not enforced.
- Everything in the repository is English – code, comments, docs, specs, ADRs, commit
  messages, PR titles and descriptions – regardless of the language used when talking
  to the agent. `CLAUDE.md` itself is written in English.
- `CLAUDE.md` in the repository describes the agents' way of working concisely and
  points to the authoritative documents; `AGENTS.md` refers to it for other agents.
- Every crate has a README with fixed sections: Responsibility, Non-Responsibility,
  Public API, Invariants, Extension Points, Tests.
- Uniform patterns instead of a variety of approaches: templates via
  `cargo xtask new-module` and `new-adapter`.
- Terms from the glossary (requirements §10) are binding.
- Small, self-contained changes with a Conventional Commit message that states the
  *why*.

## Consequences

- More up-front tooling effort (`xtask`).
- A human can join at any time: crate README → glossary → ADRs → code.
