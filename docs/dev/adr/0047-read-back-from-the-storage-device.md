# 0047 – Read back from the storage device

- Status: accepted
- Date: 2026-11-19

## Context

The verified copy reads each target back and compares its checksum with the source's
(IMPORT-004, ADR 0020 §3). The target was written and synced a moment before, so the
operating system still holds its bytes in the page cache, and the read-back is served
from memory. A USB stick or network share that stores other bytes than it was sent
passes the check, and the report calls the card safe to format (review E2).

std cannot ask the operating system to drop cached pages or to read past them. Linux
offers `posix_fadvise(POSIX_FADV_DONTNEED)`, which drops the clean cached pages of a
file; after the sync every page of the target is clean. macOS has no such call; it
offers `fcntl(F_NOCACHE)`, which turns caching off for one open file. Both need `libc`
and an `unsafe` block, which CLAUDE.md allows only in FFI adapter crates.

## Decision

### 1. A read from the storage device in the `FileSystem` port

`FileSystem` gets `open_read_from_medium`: it opens a file like `open_read`, but asks
that its bytes come from the storage device, not from memory. The verified copy uses
it for the read-back. A file system without a cache of its own, like
`MemoryFileSystem`, keeps the default, which is `open_read`. The conformance suite
checks that it reads the file's content and reports a missing file.

### 2. `adapter-fs` becomes an FFI adapter

`adapter-fs` makes the call itself, in one small module with one `unsafe` block per
call and a `// SAFETY:` comment. It becomes an FFI crate (`ffi = true`) and repeats the
workspace lints without the `unsafe` ban, as `adapter-free-space` does. A separate
adapter crate for one call was considered and rejected: the call belongs to opening a
file, and a second port would have to be wired through every copy.

- **Linux and other systems with `posix_fadvise`:** open the file, then drop its
  cached pages with `POSIX_FADV_DONTNEED` for the whole file. A failure is an error,
  so the read-back fails rather than read from memory unnoticed.
- **macOS:** open the file and set `F_NOCACHE`. This keeps the read from filling the
  cache, but pages still cached from the write may be served. macOS has no call that
  drops them for an unprivileged process, so on macOS the read-back is best effort.

### 3. `libc` in a second adapter

`libc` is allowed in `adapter-fs` too (`dependencies.yaml`). ADR 0009 wraps each
library in exactly one adapter so it can be swapped. `libc` is the binding of the C
library that std itself links; there is nothing to swap it for, and each FFI adapter
needs the declarations of its own system calls. The exception covers `libc` only.

## Consequences

- On Linux the read-back reads the device, so a device that corrupts writes is caught
  and the target is removed (IMPORT-004.15). The read-back of a large import takes as
  long as reading the files from the device, not from memory.
- `adapter-fs` now contains `unsafe` code; `check-architecture` allows it because the
  crate is marked FFI.
- No test can show on CI that the bytes came from the device; the conformance suite
  checks the content, and a test of the `files` service checks that the verified copy
  reads back through `open_read_from_medium`.
