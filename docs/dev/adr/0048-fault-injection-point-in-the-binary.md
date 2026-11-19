# 0048 – A fault-injection point in the binary

- Status: accepted
- Date: 2026-11-19

## Context

Requirements section 9.2 names faults an import must survive. CLI-022 runs T4, T7, T8
and T9 against the `framehive` binary with real adapters, because each of them can be
set up with files on disk. Four cannot:

| Scenario | Fault | Why files on disk are not enough |
|---|---|---|
| T1 | The process is killed in the middle of a copy | Killing it from the test at the right moment depends on timing |
| T3 | A bit error at the destination after writing | The read-back reads what was written; the test cannot change the bytes in between |
| T5 | A slow or hanging device | A local file system does not hang on demand |
| T6 | The device is unplugged during the import | Removing a folder is not an unplugged device; a loop device needs root |

The services already test these faults with `MemoryFileSystem` and its `Fault`s
(IMPORT-004.4, .5, .6, IMPORT-014). What is missing is a test that the binary, as
assembled in `apps/cli`, behaves the same: that the report, the exit code, the history
and the leftovers on disk are right after the fault. Tests are deterministic (CLAUDE.md
rule 4), so the fault must happen at a chosen file and byte, not at a chosen time.

## Decision

### 1. A `FileSystem` decorator in `apps/cli`

`apps/cli` gets a module `fault_injection` with `FaultyFileSystem`, a `FileSystem` that
wraps another one and passes every call through, except for the one file the fault
names. The import and the guided import wrap `StdFileSystem` in it, in
`import_ports` and where the guided import builds its ports. With no fault set, the
decorator passes `check_file_system`, the conformance suite of the port.

The decorator lives in the app, not in an adapter or in `files::testing`: it is a test
seam of the binary, it uses no library, and no other crate needs it.

### 2. Compiled only with the cargo feature `fault-injection`

The module and the wrapping exist only when `framehive-cli` is built with the feature
`fault-injection`. Without it, `import_ports` builds `StdFileSystem` as today, so the
released binary has no code that changes its behaviour from outside.

The tests of these scenarios are in `apps/cli/tests/fault_injection.rs`, compiled only
with the feature (`#![cfg(feature = "fault-injection")]`). `cargo xtask ci` gets a step
`fault injection` that runs
`cargo nextest run -p framehive-cli --features fault-injection --locked --profile ci`,
so CI runs them on every commit; the step rebuilds only `framehive-cli`. A step
`lint fault injection` before it runs clippy on `framehive-cli` with the feature, so the
code behind it meets the same lints.

### 3. One fault per run, named in `FRAMEHIVE_FAULT`

The binary reads the environment variable `FRAMEHIVE_FAULT` once at start. Its value is
`<fault> <file name>`: the fault applies to the first file with that name, at the
source or at a target, whichever the fault concerns. An unknown fault or a missing file
name is a usage error with exit code 2, so a typo never runs the import without the
fault. Without the variable, the decorator passes everything through.

| Fault | What the decorator does | Scenario |
|---|---|---|
| `abort-while-writing` | Writes half of the bytes of `<target>.part`, then calls `std::process::abort` | T1 |
| `flip-bit-after-sync` | Flips one bit of the first byte that `open_read_from_medium` returns for the target | T3 |
| `hang-while-reading` | Blocks the first read of the source file and never returns | T5 |
| `unplug-while-reading` | Fails the read of the source file and every later call below the source folder with an I/O error | T6 |

`std::process::abort` ends the process without running destructors or flushing
buffers, as a kill does, and at a fixed byte instead of a moment the test guesses. The
test of T1 then runs the import again without the fault and checks that it resumes.

An environment variable is not configuration in the sense of ADR 0011: no user sets it,
it has one value, and it exists only in a test build.

### 4. Behaviour first, then the test

The decorator only causes faults; what the import does about them is in the specs. T1,
T3 and T6 are covered by rules that exist (IMPORT-004, IMPORT-009, IMPORT-014). T5 needs
a per-file time limit that no spec defines yet; it gets its own spec, and an ADR if it
needs a thread or a native call, before `hang-while-reading` is built. The scenarios
get criteria in a new spec beside CLI-022, each marked as usual (ADR 0010).

## Alternatives considered

- **Always compiled, enabled by the variable.** Simplest, but every released binary
  would carry a switch that aborts it or corrupts a read-back. Rejected.
- **Compiled in debug builds only** (`cfg(debug_assertions)`). Needs no extra CI step,
  but ties a test seam to an optimization profile, and a release build with debug
  assertions would carry it unnoticed. Rejected in favour of a named feature.
- **A second binary for the tests.** It would not be the binary that ships, and
  `apps/cli` would have to become a library. Rejected.
- **Killing the process from the test** (T1). Needs a wait for the copy to reach a
  point, which depends on timing. Rejected.
- **A FUSE or loop-device file system** (T5, T6). Needs root or a native library on CI
  and on macOS. Rejected.

## Consequences

- T1, T3 and T6, and T5 once its time limit is specified, run against the binary with
  real adapters, deterministically.
- `apps/cli` gets a feature and one extra test step; `cargo xtask ci` and CI take as
  long as one more build of `framehive-cli` and these tests.
- A test that needs a fault outside the `FileSystem` port, such as in the history or
  in a camera, needs another decorator and an amendment of this ADR. Faults of a camera
  source are left to the virtual camera of libgphoto2 (requirements section 9.3).
