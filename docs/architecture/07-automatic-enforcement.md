# 7. Automatic enforcement (fitness functions)

Part of the [architecture](README.md).

| Rule | Check in CI |
|---|---|
| Layers & module isolation (Section 3) | `cargo xtask check-architecture`: reads `cargo metadata` and the `layer` annotations, fails on forbidden edges |
| Domain has no I/O | Clippy `disallowed_methods`/`disallowed_types` per layer (`SystemTime::now`, `Instant::now`, `std::fs`, `thread::sleep`, …) + dependency allowlist |
| No accidental public API changes | `cargo public-api` snapshot for `kernel` and services; a change requires an updated snapshot and a changelog entry |
| No `unsafe` outside FFI adapters | `#![forbid(unsafe_code)]` in all other crates, checked by `check-architecture` |
| Documentation | `#![warn(missing_docs)]` treated as an error in CI; every crate has a README with mandatory sections (checked by `xtask`) |
| Formatting & lints | `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` with `clippy::pedantic` (justified exceptions in `clippy.toml`) |
| Dependencies | `cargo xtask check-dependencies` against the allowlist `dependencies.yaml`, including permitted layers; count of transitive dependencies logged; `cargo deny` (licenses, duplicates, advisories, crates.io only); unused dependencies (ADR 0009) |
| Size and complexity | `cargo xtask check-files`: at most 400 lines per Rust file; `clippy::too_many_lines`: 60 lines per function. Splitting is mandatory |
| Document size | `cargo xtask check-files`: Markdown files at most 10 KiB (ADRs and `CHANGELOG.md` exempt); longer documents become a directory with one file per section and a `README.md` index |
| Specification ↔ tests ↔ code in sync | `cargo xtask check-specs` (ADR 0010) |
| Configuration schema | JSON Schema generated from the typed config, checked for drift; all example configs validate (ADR 0011) |
