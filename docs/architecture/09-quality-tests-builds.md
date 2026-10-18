# 9. Quality, tests, builds

Part of the [architecture](README.md).

## 9.1 Testability by design

- Business logic (domain, use cases, ViewModels) is pure and is fully covered by unit
  and property tests; target ≥ 90% line coverage, mutation score (`cargo mutants`)
  ≥ 80% in `domain`, `naming`, `metadata`, `import`.
- Adapters are tested against their port's conformance suite, with real resources
  (temp directories, SQLite files, virtual camera).
- End-to-end via the CLI and via the GUI, as described in [requirements §9](../requirements/09-test-strategy-and-quality-assurance.md).

## 9.2 No flakiness

| Cause | Countermeasure |
|---|---|
| Time | `Clock` port; tests use `ManualClock`. `sleep` in tests is forbidden by lint |
| Concurrency | Job scheduler with a swappable executor; tests use a deterministic step executor. Wait only on events, never on time |
| Randomness | Fixed seeds; proptest regressions are checked in |
| Ordering | `cargo nextest` (every test in its own process); hash-map output sorted before comparison, or `BTreeMap` |
| Environment | Tests set `TZ`, locale and home directory explicitly; no access to the real home, network (blocked in CI) or system devices |
| File system | Each test gets its own temp directory; paths are never hard-coded |
| UI | Software renderer, embedded fonts, fixed window size and scaling for screenshot tests; expected results are pixel hashes (ADR 0017) |
| Test data | Sample files in the separate `framehive-testdata` repository, pinned with BLAKE3 in `testdata.lock`, cached in `.testdata/` (ADR 0017) |

Policy: **No retries in CI.** A flaky test is a bug with the highest priority.
Nightly runs the suite 20 times in random order to catch flakes early.

## 9.3 Reproducible builds and tests

- `rust-toolchain.toml` pins the Rust version and components; `Cargo.lock` is
  checked in; all commands use `--locked`.
- System dependencies (libgphoto2, ffmpeg, ONNX Runtime) and tools come from a
  **Nix flake** with a lockfile, identical locally and in CI, on Linux and macOS.
- `SOURCE_DATE_EPOCH`, `--remap-path-prefix`, no embedded build times or hostnames.
  CI builds Linux releases twice and compares the checksums.
- One entry point for everything: `cargo xtask ci` runs exactly what CI runs.

## 9.4 Development process

- Trunk-based: small, self-contained changes; `main` is always green and
  release-ready.
- Every change: tests first or at the same time, a changelog entry for user-visible
  changes, a Conventional Commits message.
- Architecture decisions as an ADR (Section 11), before code is written.
- "Done" means: `cargo xtask ci` green, documentation updated, no new exceptions
  without justification.
- Language: everything in the repository is English – code, comments, docs, specs,
  ADRs, commit messages, PR titles and descriptions – regardless of the language used
  when talking to the agent.
- Every behaviour change starts with the specification (ADR 0010).
