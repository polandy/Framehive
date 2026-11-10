<!--
SPDX-FileCopyrightText: 2026 Framehive contributors
SPDX-License-Identifier: GPL-3.0-or-later
-->

# adapter-macos-trash

Layer: `adapter`, port `files` (architecture §3), FFI adapter

## Purpose

Implements the `Trash` port of `files` on macOS with
`-[NSFileManager trashItemAtURL:resultingItemURL:error:]`, the call Finder uses, so
"Put Back" restores a trashed file (ADR 0043). Remove RAWs and deleting in the library
use it on macOS. The crate is empty on other systems.

- **Native code.** It declares the few functions it calls itself: the Objective-C
  runtime (`objc_getClass`, `sel_registerName`, `objc_msgSend`, the autorelease pool)
  and CoreFoundation's CFURL, CFError and CFString functions, to which NSURL and NSError
  are toll-free bridged. It links the system frameworks Foundation and CoreFoundation;
  there is no build script and no binding crate.
- **`unsafe`:** every block holds one call with a `// SAFETY:` comment, all in
  `src/ffi.rs`. `objc_msgSend` is only called through a pointer of the exact method type.
- **Errors.** Folders and symbolic links are refused before the call. Cocoa file errors
  map to `NotFound`, `PermissionDenied` and `StorageFull`; any other error is `Other`
  with the system's description.
- **Not covered here:** restoring and emptying the Trash, which Finder does.

## Public API

| Item | Meaning |
|---|---|
| `MacosTrash` | `files::Trash` through `NSFileManager`, on macOS only |

## Conformance

`tests/conformance.rs` runs `files::testing::check_trash` on macOS. `trashItemAtURL`
cannot be pointed at another folder, so the test moves files from a `TempDir` to the
Trash of the user who runs it and removes each one from there by the path the call
returned (ADR 0043 §4). It also checks that a symbolic link is refused.

## Dependencies

- `files`
- System frameworks Foundation and CoreFoundation, and `libobjc` (ADR 0043)
