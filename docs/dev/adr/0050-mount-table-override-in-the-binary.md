# 0050 – A mount table override in the binary

- Status: accepted
- Date: 2026-11-19

## Context

The guided import (CLI-002, CLI-003, CLI-006, CLI-011, CLI-017) is tested through
`Dialog` with fake ports. No test runs the `framehive` binary through it, so the
wiring of the real adapters in `run_guided` is never run by a test. Two things stand in
the way (issue #167):

1. The guided import refuses to start unless standard input and standard output are
   terminals (CLI-002.1), so a test cannot pipe a script into it.
2. `MountsDeviceSource::new()` reads `/proc/self/mountinfo` and considers only mounts
   below `/run/media/` and `/media/`. A test cannot mount a card there without root.

`MountsDeviceSource::linux(mountinfo, folders)` already reads any mount table file and
any folders; its own tests use it with a written file.

## Decision

### 1. Two environment variables, read by the guided import

On Linux, `run_guided` reads two environment variables before it builds its ports:

| Variable | Value |
|---|---|
| `FRAMEHIVE_MOUNTINFO` | The file read as `/proc/self/mountinfo` |
| `FRAMEHIVE_MEDIA_FOLDERS` | The folders whose mounts are considered, separated by `:` as in `PATH` |

With both set, the guided import builds `MountsDeviceSource::linux(file, folders)`.
With neither set, it builds `MountsDeviceSource::new()`, so nothing changes. With only
one set, it prints `error: FRAMEHIVE_MOUNTINFO and FRAMEHIVE_MEDIA_FOLDERS are set
together or not at all` and exits with code 2, so a typo never runs against the real
mount table. On macOS both are ignored; `/Volumes` holds plain folders, and the dialog
tests cover the rest.

A test then writes a mount table line whose mount point is a folder laid out like a
card, below a folder it names in `FRAMEHIVE_MEDIA_FOLDERS`. The binary sees that folder
as a card, with real files, a real history and the real free space.

The eject runs `udisksctl` on the mount source of that line, as for a real card; the
tests answer `n` to `Eject <name>? [Y/n]`, so they never start it.

### 2. Always compiled

Unlike `FRAMEHIVE_FAULT` (ADR 0048), the variables are in every build. They only change
which file and which folders list the cards, which a person could achieve with a bind
mount; they cannot make an import abort, lose or corrupt a file. So the tests run in
the ordinary test step, and no feature is needed.

An environment variable is not configuration in the sense of ADR 0011: no user sets it,
and it exists for the tests.

### 3. A pseudo-terminal in the test, with `rustix`

The tests in `apps/cli/tests/guided_import_runs.rs` start the binary with a
pseudo-terminal as standard input and output and a pipe as standard error, write the
script to the terminal and read its output until the binary exits. They open the
terminal with `rustix` (feature `pty`), a dev-dependency of `framehive-cli` only. Its
API is safe, so the tests need no `unsafe`, which the workspace forbids outside the FFI
adapters (architecture §7). `libc` would need `unsafe` for `openpty`, which `forbid`
does not let a test file allow; the owner chose `rustix` over that and over the
`script` tool of util-linux.

## Alternatives considered

- **A hidden `--script <file>` option that skips the terminal check.** Smaller tests,
  but a way around CLI-002.1 in every build. Rejected (#167, option B).
- **Only the dialog tests.** The wiring of the real adapters would stay untested.
  Rejected (#167, option C).
- **Behind the feature `fault-injection`.** The variables cause no fault, and the
  tests would run only in the step of ADR 0048. Rejected.
- **`script` from util-linux** as the terminal. No dependency, but a tool outside the
  Nix shell whose end-of-input handling differs between versions. Rejected.

## Consequences

- One binary run per guided-import spec covers the wiring of `run_guided` with real
  adapters on Linux. The macOS wiring stays covered only by the dialog tests.
- `rustix` and `linux-raw-sys` enter `Cargo.lock`, for the tests only.
- The released binary reads two more environment variables in the guided import.
