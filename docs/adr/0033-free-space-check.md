# 0033 – Free-space check before an import

- Status: accepted
- Date: 2026-11-05

## Context

An import must not start when a destination cannot hold the files it is about to write
(I1, IMPORT-012). A destination that fills up during a copy is already handled: the
`.part` file is removed and the file fails (IMPORT-004). But a run that stops halfway
leaves a card that is half imported and half backed up, and the person learns it only
at the end.

The Rust standard library cannot tell how much space a file system has free. The
answer comes from `statvfs(3)`, which Linux and macOS both provide. Calling it needs
the `libc` crate and an `unsafe` block, which CLAUDE.md allows only in FFI adapter
crates. `libc` is already in `Cargo.lock` as a transitive dependency, but not a direct
one in `dependencies.yaml`.

Two destinations can lie on the same file system, for example the library and a backup
folder on one external disk. Checking each against the free space on its own would let
both pass while together they do not fit.

## Decision

### 1. A `FreeSpace` port in the `files` service

The `files` service gets a port `FreeSpace` with one query: for a folder path, the
bytes an unprivileged process may still write (`f_bavail × f_frsize`) and an identity
of the file system the path lies on (`st_dev` of the path). A path that does not exist
yet is answered for its nearest existing parent folder, because the import creates
missing destinations itself. The port has an in-memory implementation with fixed
answers for tests and a conformance suite.

### 2. One FFI adapter crate with `libc`

A new adapter crate `crates/adapters/free-space` implements the port with
`libc::statvfs` and `std::os::unix::fs::MetadataExt::dev`. It holds the only `unsafe`
block, with a `// SAFETY:` comment. `libc` becomes a direct dependency, wrapped in this
one crate. It binds the C library that std itself links, so it adds no native build
step.

### 3. The check adds up destinations per file system

Before the first copy, the import adds up, per file system, the bytes it will write to
each destination on it, and compares the sum with that file system's free space. There
is no reserve on top: `f_bavail` already leaves out the blocks reserved for root, and a
disk that still fills up is handled by IMPORT-004.

### 4. An unknown answer does not block

When the free space of a destination cannot be read, for example on a network file
system that does not report it, the import warns and goes on. There is no option to
skip the check.

## Alternatives

- **`rustix`:** a safe API without our own `unsafe`, but more transitive crates
  (`bitflags`, `linux-raw-sys`, and `libc` again on macOS) for one system call.
- **Run `df` and parse its output:** spawns a process, and the output format differs
  between Linux and macOS.
- **Add the query to the `FileSystem` port:** every `FileSystem` implementation would
  need it, and the `fs` adapter would lose its `forbid(unsafe_code)`.
- **Check each destination on its own:** simpler, but passes two destinations on one
  disk that do not fit together.
- **A fixed reserve (1 % or 100 MB):** would block imports that fit, with no gain over
  the clean stop of IMPORT-004.

## Consequences

- The check is as exact as the file sizes and the free space the system reports. Block
  rounding and folder entries can still fill a disk that was reported just big enough;
  IMPORT-004 then stops that file cleanly.
- APFS volumes that share one container report different `st_dev` values but the same
  free space, so two destinations on such volumes are checked on their own. This can
  pass an import that does not fit; IMPORT-004 then stops it cleanly.
- The check reads the free space once, before the run. Another program that writes to
  the disk during the import is not taken into account.
