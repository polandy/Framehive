# 0037 – Coverage-guided fuzzing with cargo-fuzz

- Status: accepted
- Date: 2026-11-07

## Context

The EXIF reader in `metadata` is our own parser of untrusted bytes (ADR 0021). ADR 0009
accepts our own parser only when it is fuzzed, and ADR 0021 §4 left the tool open until
the code exists. Requirements §9.5 names `cargo-fuzz` for a nightly run.

Today a deterministic mutation test stands in (`crates/services/metadata/tests/mutated_files.rs`,
CORE-005.9 and .12): it builds valid files, damages them with a seeded PRNG and checks
that the reader neither panics nor reads too much. It runs on every PR, but it only
tries the damage it was written to make. A coverage-guided fuzzer keeps the inputs that
reach new branches and so finds paths nobody thought of, such as a crafted IFD chain
inside the RAF's embedded JPEG.

`cargo-fuzz` drives LLVM's libFuzzer through the `libfuzzer-sys` crate, which compiles
the libFuzzer C++ sources with the C++ compiler during the build. That is native code,
so ADR 0009 §4 asks for this ADR. Three more points need a decision:

- The sanitizer and coverage flags that `cargo-fuzz` passes to `rustc` are unstable,
  so it needs a nightly toolchain, while the project pins a stable one
  (`rust-toolchain.toml`, ADR 0007).
- A fuzzing run is not deterministic; its result depends on time and the machine.
  Rule 4 of CLAUDE.md does not allow that in a test.
- A fuzzing corpus is a folder of binary files, and ADR 0017 keeps binary files out of
  the repository.

## Decision

### 1. cargo-fuzz with libFuzzer

The EXIF reader is fuzzed with `cargo-fuzz` and `libfuzzer-sys`. `libfuzzer-sys` is a
dependency of the fuzz crate (§2) only, never of a library or app. It is listed in
`dependencies.yaml` with this ADR, allowed only in the fuzz crate. Its license is
`(MIT OR Apache-2.0) AND NCSA`; NCSA (the old LLVM license, permissive and compatible
with the GPL) is added to `deny.toml` for this crate. No `arbitrary` crate: the targets
take raw bytes, which is what a file is.

### 2. One fuzz crate, outside the workspace

The targets live in `fuzz/` at the repository root, in a crate `metadata-fuzz` with its
own `[workspace]` and its own `Cargo.lock`, which is committed and used with `--locked`.
The main workspace leaves it out, so `cargo xtask ci`, `Cargo.lock` and the release
build never see libFuzzer. `check-dependencies`, `check-files` and `cargo deny` also
check `fuzz/`, so the crate follows the same rules as every other one.

The first target, `capture_time`, passes the input to `metadata::capture_time_from_bytes`.
The parser is a pure function of the bytes (ADR 0021 §3), so the target needs no port.
A panic, an overflow or an allocation above libFuzzer's limit is a finding. More
targets (XMP, maker notes) join the same crate when their parsers exist.

### 3. A pinned nightly toolchain in a separate Nix shell

The flake gets a second development shell, `nix develop .#fuzz`, with a nightly
toolchain pinned to a date and `cargo-fuzz` from nixpkgs. The default shell, CI and the
release build keep the stable toolchain from `rust-toolchain.toml`. The nightly date is
updated like any other pin, by a PR.

### 4. Fuzzing runs nightly, outside `cargo xtask ci`

`cargo xtask fuzz [--seconds <n>]` builds the corpus seeds (§5) and runs each target
for the given time (default 600 seconds). A scheduled GitHub Actions workflow runs it
once a night on Linux, in the fuzz shell. The workflow only calls the xtask command,
like `ci.yml`. It is not a required check and never blocks a PR, because its result is
not deterministic. A failing run stores the crashing input and the log as a CI
artifact.

The mutation test stays. It is the deterministic gate on every PR; the fuzzer searches
further at night.

### 5. Seeds are built by code, findings become tests

The corpus is not committed (ADR 0017). `cargo xtask fuzz` writes the seeds into the
ignored `fuzz/corpus/` before a run, built by the same `metadata::testing` builders the
tests use: JPEG, TIFF and RAF, both byte orders, with and without offset and sub-seconds.
The nightly workflow keeps the grown corpus between runs in the Actions cache, so the
fuzzer does not start from scratch.

Each finding is fixed by a deterministic regression test in `metadata` that builds the
crashing bytes in code or writes them as a short byte array, never as a file. The test
carries the `spec:` marker of the criterion the crash broke (CORE-005.9 or .12).

## Alternatives

- **Only the mutation test:** deterministic and already there, but it only finds the
  damage it was written to make. ADR 0009 asks for fuzzing.
- **`cargo-afl` (AFL++):** also coverage-guided and stable Rust, but it needs its own
  compiler wrapper and a forking runner, and its crate builds AFL++'s C sources just as
  `libfuzzer-sys` builds libFuzzer. `cargo-fuzz` is the tool the Rust fuzzing
  organization maintains and the requirements name.
- **`honggfuzz`:** works on Linux but poorly on macOS, and builds a larger C project.
- **`bolero`:** one harness for fuzzing and property tests, but a larger dependency
  tree, still on libFuzzer for coverage, and more abstraction than one byte target needs.
- **Nightly for the whole project:** one toolchain fewer, but the release build would
  depend on unstable compiler versions (ADR 0007).
- **Fuzzing in every PR for a short time:** a 30-second run finds little, adds a
  nondeterministic step to the required checks and can turn a PR red for a bug it did
  not cause.

## Consequences

- One native build dependency more, in a crate the product never links.
- Two toolchains in the flake. The nightly one is downloaded only in the fuzz shell.
- A finding shows up a day after the change that caused it, not in the PR. The
  regression test makes sure it stays fixed.
- The nightly run needs GitHub Actions minutes on Linux: about ten minutes per target.
- `TODO.md` tracks the implementation of this ADR: the fuzz crate, the shell, the xtask
  command and the workflow.
