# Framehive – Requirements

**Framehive** is a modular photo library for Linux and macOS, written in
Rust. The first module is an import feature modeled on
[Rapid Photo Downloader](https://damonlynch.net/rapid/) (RPD); the core of the
application is a library for viewing, tagging, and culling, similar to digiKam.

Status: Draft v0.3 (2026-10-17)

Legend: **[RPD]** = feature adopted from Rapid Photo Downloader ·
**[NEW]** = improvement proposal · Priority **M** = MVP, **S** = should (v1),
**K** = could (later)

Detailed, binding behaviour of each feature is specified in `specs/` (ADR 0010). This
document is the catalogue: what, why, priority.

One file per section, so that a reader opens only the section a change touches.
Section numbers are stable and are cited as `requirements §<n>`. Spec origins refer to
requirement IDs and section numbers in these files.

| § | Section |
|---|---|
| 1 | [Goal & Scope](01-goal-and-scope.md) |
| 2 | [Import module: adopted from RPD](02-import-module-adopted-from-rpd.md) |
| 3 | [Weaknesses of RPD](03-weaknesses-of-rpd.md) |
| 4 | [Improvement Proposals [NEW]](04-improvement-proposals.md) |
| 5 | [Library module](05-library-module.md) |
| 6 | [Storage locations, multiple machines, migration](06-storage-locations-multiple-machines-migration.md) |
| 7 | [Non-functional requirements](07-non-functional-requirements.md) |
| 8 | [Architecture sketch](08-architecture-sketch.md) |
| 9 | [Test strategy & quality assurance](09-test-strategy-and-quality-assurance.md) |
| 10 | [Roadmap](10-roadmap.md) |
| 11 | [Open questions](11-open-questions.md) |
| 12 | [Decisions](12-decisions.md) |
