# 0043 – Trash port: the freedesktop.org trash and the macOS Trash

- Status: accepted
- Date: 2026-11-10

## Context

Remove RAWs moves RAW files and their own sidecars to the trash (LIBRARY-002, B30), and
deleting in the library goes to the trash too (B13, LIBRARY-001). The trash is the only
way back from both, so a file must land where the desktop can restore it, and a failure
must leave the file where it was (LIBRARY-002.15–.17).

The Rust standard library has no trash. Linux desktops follow the freedesktop.org Trash
specification 1.0 (architecture §8): a `files` folder with the file and an `info` folder
with a `.trashinfo` file naming its original path and deletion time, in the home trash
or, for a file on another file system, in a trash at that file system's top folder. The
specification is short and needs only `std`.

macOS has no such specification. Moving a file to `~/.Trash` by hand works, but Finder's
"Put Back" then does not know where the file came from, and a file on another volume
would be copied across volumes. Only `-[NSFileManager trashItemAtURL:resultingItemURL:error:]`
(Foundation) moves a file the way Finder does. Calling it needs the Objective-C runtime
and native frameworks. The owner chose the native call under its own ADR (issue #109,
2026-11-10).

## Decision

### 1. A `Trash` port in the `files` service

`files` gets the port `Trash` with one method: `move_to_trash(path)` moves one file and
returns where it lies in the trash. A file of the same name already in the trash stays;
the new one gets a name of its own. Nothing at `path` is `NotFound`; a folder, a symbolic
link or anything else that is not a file is refused with `InvalidInput`, because the
library trashes asset files only. On every error the file stays. `testing::MemoryTrash`
moves files within a `MemoryFileSystem` and can refuse chosen files, for the failure
paths of Remove RAWs. `testing::check_trash` is the conformance suite; it lays out and
reads its files through functions of the implementation's storage and removes every
file it trashed again.

### 2. `adapter-freedesktop-trash`: own code, `std` only

The adapter implements specification 1.0 in own code (ADR 0009) and takes the home trash
folder, `$XDG_DATA_HOME/Trash`, from the app, and a `Clock` for the deletion time:

- a file on the file system of the home trash goes there, with an absolute `Path`;
- a file on another file system goes to `$topdir/.Trash/<uid>` if `.Trash` is a real
  folder with the sticky bit, else to `$topdir/.Trash-<uid>`, with `Path` relative to
  the top folder; the uid is the owner of `/proc/self`. Copying it to the home trash
  instead is not done: a copy across file systems is not a move, and a failure halfway
  would leave two files;
- the `.trashinfo` file is created with `create_new` before the move, which reserves the
  name against other programs; a name taken in `info` or `files` gets a number before
  the extension, `DSCF1234.2.RAF`; if the move fails, the info file is removed again;
- `Path` is percent-encoded, `DeletionDate` is local time without a time zone.

`directorysizes` is not written, because the port moves no folders.

### 3. `adapter-macos-trash`: native `trashItemAtURL`

An FFI adapter crate declares the few functions it needs itself, without a binding crate:
`objc_getClass`, `sel_registerName`, `objc_msgSend` and the autorelease pool from
`libobjc`, and the CFURL, CFError and CFString functions from CoreFoundation; NSURL and
NSError are toll-free bridged to them. It links the system frameworks Foundation and
CoreFoundation, which every macOS ships, so there is no build step and no new entry in
`dependencies.yaml`. Every `unsafe` block holds one operation with a `// SAFETY:`
comment. The crate is empty on other systems. Cocoa file errors map to `NotFound`,
`PermissionDenied` and `StorageFull`, everything else to `Other` with the system's
description.

### 4. The macOS conformance test uses the user's Trash

`trashItemAtURL` cannot be pointed at another folder. The macOS conformance test
therefore moves files from a temporary folder to the Trash of the user who runs it and
removes each one from there at the end, by the path the call returned. It touches no
other file of the home folder. This is the one test that reaches outside a temporary
folder; the alternatives below cost more than they protect.

## Alternatives

- **Move to `~/.Trash` by hand on macOS:** no native code, but no "Put Back", and a file
  on another volume would be copied. Rejected by the owner (issue #109).
- **The `trash` crate:** covers both systems, but brings `objc2` and its framework crates
  on macOS and more code than the port needs on Linux; ADR 0009 prefers own code for a
  short specification.
- **`objc2` and `objc2-foundation` for the macOS call only:** safe wrappers, but a large
  family of crates for two messages.
- **`FSMoveObjectToTrashSync` (CoreServices):** plain C, but deprecated since macOS 10.8.
- **Run `osascript` to ask Finder:** spawns a process, needs automation permission, and
  is slow for hundreds of RAW files.
- **Test the macOS adapter on a disk image** made with `hdiutil`: leaves the home Trash
  alone, but needs an external tool and a mount in every test run.

## Consequences

- Remove RAWs and deleting in the library can trash files on Linux and macOS, and the
  desktop restores them.
- The library and its failure-path tests use `Trash` and `MemoryTrash` without knowing
  the adapter; only apps pick one per system.
- `adapter-macos-trash` is built and tested on macOS only, in CI's macOS job.
- A trash at a file system's top folder that cannot be created is an error, and the
  file stays.
