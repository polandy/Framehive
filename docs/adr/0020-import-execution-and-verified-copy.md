# 0020 – Import execution, verified copy, `blake3` and `clap`

- Status: proposed
- Date: 2026-10-31

## Context

Milestone 1 is a real import from the command line: `framehive import <source>
<destination>` scans a folder, makes the import plan (IMPORT-001) and copies every file
to its target path. Until now the import module only plans; nothing reads or writes
files.

Copying photos is the one step where data can be lost. The requirements ask for a
verified copy with a checksum (N1), atomic writes through a `.part` file (N2), never
writing to the source (N7), and that one broken file never aborts the import (N26).

The architecture describes an import as a job in a queue (J1) that reports through
kernel events. Neither the jobs service nor the kernel's module contract exists yet.
Building them first would delay the first usable import by several tasks, and their
design is better informed by a working import.

## Decision

### 1. A synchronous import run, no job yet

The import module gets an `ImportRun`: it takes an import plan and executes it file by
file, through ports only, and reports each step to a `ProgressSink` trait that the
module defines. It runs on the caller's thread.

There are no new kernel events or commands. When the jobs service exists, `ImportRun`
becomes the body of the import job, and its progress is mapped to kernel events; that
step gets its own ADR.

### 2. Copy, never move, never overwrite

An import copies. It never deletes, renames or writes anything in the source. It never
replaces an existing file at the destination: conflicts are resolved by the conflict
policy (IMPORT-003) before a file is copied, and a file that appears at the target path
during the copy is kept, and the copy is reported as failed.

### 3. Verified copy

Each file is copied as IMPORT-004 specifies:

1. Read the source once, hash the bytes while writing them to `<target>.part` next to
   the target.
2. Flush and `fsync` the `.part` file.
3. Rename it to the target without replacing an existing file, then `fsync` the folder.
4. Read the target back, hash it, and compare it with the source checksum. On a
   mismatch, remove the target and report the file as failed.

The rename never replaces. `adapter-fs` makes it atomic with a hard link to the target
followed by removing the `.part` file. Where the file system has no hard links (exFAT,
some SMB shares), it checks that the target does not exist and renames; the remaining
race with another program writing the same name is accepted and documented.

### 4. New crates and port

| Crate | Layer | Content |
|---|---|---|
| `domain` (existing) | domain | `Clock` port, `Timestamp` (an instant in UTC), `UtcOffset`, `Checksum` (algorithm and digest, written `blake3:<hex>`, architecture §8), `testing::ManualClock` |
| `naming` (existing) | service | `LocalDateTime::from_timestamp`, to name files by their modification time in local time |
| `files` (`crates/services/files`) | service | Ports `FileSystem` and `Hasher`; `copy_verified`; `testing` with `MemoryFileSystem`, `FnvHasher`, `TempDir` and the conformance suites of both ports |
| `metadata` (`crates/services/metadata`) | service | Reading the capture time; own EXIF reader, ADR 0021 |
| `adapter-fs` (`crates/adapters/fs`) | adapter, port `files` | `FileSystem` over `std::fs` |
| `adapter-blake3` (`crates/adapters/blake3`) | adapter, port `files` | `Hasher` with BLAKE3 |
| `adapter-clock` (`crates/adapters/clock`) | adapter | `Clock` from `domain`, with the system time zone; ADR 0022 |
| `framehive-cli` (`apps/cli`) | app | The binary `framehive`; wires the adapters, parses arguments (CLI-001) |

`MemoryFileSystem` can drop every write that was not synced, so tests can check that a
file reported as copied survives a crash. `FnvHasher` is a small, non-cryptographic
hasher for tests that need no real checksum. `TempDir` is our own helper for tests that
touch a real file system, instead of the `tempfile` crate.

### 5. `blake3` for checksums

Checksums use BLAKE3, the algorithm architecture §8 already names. The `blake3` crate
is used only in `adapter-blake3`.

| Criterion (ADR 0009 §3) | `blake3` |
|---|---|
| Origin | Official implementation by the BLAKE3 authors (`BLAKE3-team`) |
| Adoption | The reference crate for BLAKE3 and among the most downloaded hash crates |
| Maintenance | Regular releases |
| Stability | Version 1.x |
| License | CC0-1.0 OR Apache-2.0 (OR Apache-2.0 WITH LLVM-exception) |
| Scope | `default-features = false`, feature `std` |

A cryptographic hash function is a problem ADR 0009 §2 assigns to a library. The crate
contains assembly and C for its SIMD paths, built by its build script; that native code
is the reason for this ADR (`CLAUDE.md`, rule 6). Its feature `pure` drops it, at some
speed cost, if it ever breaks the reproducible build (ADR 0007).

### 6. `clap` for the command line

The argument parser is `clap`, used only in `framehive-cli` (ADR 0009 §5), with the
builder API: the derive API would add a proc-macro crate for a handful of arguments.

| Criterion (ADR 0009 §3) | `clap` |
|---|---|
| Origin | `clap-rs` organization, several active maintainers |
| Adoption | The most used argument parser in Rust, used by Cargo and rustup |
| Maintenance | Regular releases |
| Stability | Version 4.x |
| License | MIT OR Apache-2.0 |
| Scope | `default-features = false`, features `std`, `help`, `usage`, `error-context` |

## Alternatives

- **The import as a job with kernel events now:** the target design, but it needs the
  jobs service, the module contract and the event bus first. Deferred, not dropped.
- **Copy with `std::fs::copy` and compare afterwards:** reads the source twice and
  leaves a partial file under the final name when interrupted (against N2).
- **`xxh3` instead of BLAKE3:** faster, but not cryptographic. Content-based
  recognition (N3) and the import log (N6) should not rest on a hash with constructible
  collisions, and BLAKE3 is fast enough to be limited by the disk.
- **Our own BLAKE3:** security-critical and easy to get subtly wrong (ADR 0009 §2).
- **`tempfile` in tests:** an extra dependency for about 50 lines of our own code.
- **Our own argument parser, or `lexopt`/`pico-args`:** less code to depend on, but
  help text, error messages and subcommands would be ours to maintain, and readers
  expect `clap`.

## Consequences

- The first end-to-end import works before the jobs service exists; moving it into a
  job later changes the import module and `framehive-cli`, not the copy itself.
- Every write at the destination goes through `copy_verified`; no other code path
  writes imported files.
- Each file is read twice from disk (source once, target once), the price of a
  verification that catches write errors.
- `dependencies.yaml` gains `blake3` and `clap`; architecture §2 (overview), §10
  (glossary) and §12 (repository structure) name the new crates and terms.
