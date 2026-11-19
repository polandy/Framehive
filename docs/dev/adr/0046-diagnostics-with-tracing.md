# 0046 – Diagnostics with tracing

- Status: accepted
- Date: 2026-11-19

## Context

CLAUDE.md forbids `println!` in libraries and names `tracing` instead, and
architecture §6 asks for structured diagnostic events with spans per job and file.
Neither existed: no crate depended on `tracing`, and `--verbose` did nothing (review
E28). A hanging camera import, a rename that fell back to the path without hard links
(ADR 0020 §3) or a volume that could not be read (review E17) left no trace.

ADR 0009 lists `tracing` and `tracing-subscriber` as the logging candidates. It also
says that an external library is wrapped in exactly one adapter. A logging facade
cannot be wrapped that way: every crate that has something to report must emit to it.

## Decision

### 1. `tracing` is a facade every layer but `domain` may use

`tracing` is allowed in `kernel`, services, modules, adapters and apps, like
`thiserror` (`dependencies.yaml`). It is built without the `attributes` feature, so
there are no proc-macro crates and no `#[instrument]`; spans are written with
`info_span!`. `domain` stays free of it: domain values have nothing to report.

### 2. Only the app subscribes

`tracing-subscriber`, with only the `fmt` feature, is allowed in `framehive-cli`
alone. Libraries only emit; without a subscriber an event costs a check of a static
level. `apps/cli/src/diagnostics.rs` sets the subscriber once at start.

### 3. Standard error, by level, without a time

The subscriber writes one line per event to standard error, so the report on standard
output stays the same (CLI-021). Without `--verbose` it shows warnings and errors;
each `--verbose` shows one level more: information, details, everything. The line has
no time, so that two runs compare and tests can match it; there are no colours, as the
`ansi` feature is off.

### 4. What is logged

| Level | Event |
|---|---|
| warning | an entry of `/Volumes` or a `DCIM` folder that cannot be examined (E17) |
| information | each import job (span `import_job`, field `source`) and each file (span `file`, field `path`) with its outcome; the end of the job; the first report whose enqueuer is gone (E13); a camera list that fails while a port names the camera (E17) |
| detail | a rename without a hard link (ADR 0020 §3, E7) |

New events follow the same levels: a warning for something the user should fix, an
information line per unit of work, a detail for a fallback or a decision.

### 5. Not yet

No log file and no diagnostic export; no filter by crate or `RUST_LOG`, which would
need the `env-filter` feature and the regex crates. They can follow when a fault report
asks for them.

## Consequences

- Eight crates join the lock file: `tracing`, `tracing-core`, `tracing-subscriber`,
  `sharded-slab`, `thread_local`, `pin-project-lite`, `once_cell` and `lazy_static`,
  all pure Rust.
- Every crate may report what it does without knowing where it goes; the tests of the
  app check the levels and the line format (CLI-021).
- Events are not part of the specified behaviour apart from CLI-021; their wording may
  change without a spec change.
