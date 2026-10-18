# 0001 – Record architecture decisions as ADRs

- Status: accepted
- Date: 2026-10-17

## Context

Framehive will likely be developed exclusively by AI agents. Agents have no memory
across sessions; humans must still be able to understand why something was built the
way it was.

## Decision

Every decision that affects structure, interfaces between crates, data formats,
standards mappings or tools is recorded **before** implementation as an ADR in MADR
format under `docs/adr/`. Accepted ADRs are never changed afterwards; a new ADR
supersedes them ("superseded by 00NN").

## Consequences

- The rationale lives next to the code and is versioned.
- Agents read the relevant ADRs before architectural changes (see `CLAUDE.md`).
- Slightly more effort per decision.
