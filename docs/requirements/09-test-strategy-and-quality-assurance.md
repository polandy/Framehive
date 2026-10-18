# 9. Test strategy & quality assurance

Part of the [requirements](README.md).

The biggest risks aren't in the UI, but in copying, verifying, devices, and
interruptions. Hence a test pyramid weighted toward the core.

## 9.1 Levels

| Level | Content | Tool | Runs |
|---|---|---|---|
| Unit and property tests | Template engine, sequences, timezones and camera time correction, conflict detection, RAW+JPG pairing | `cargo test`, `proptest` | every commit |
| Snapshot tests | Import plan (source → destination, conflicts, space needed) as a snapshot | `insta` | every commit |
| **End-to-end test of the core** | Real imports via the headless CLI (N18) against memory-card images and a virtual camera, with deliberately injected faults (9.2) | `cargo test` + test data, CI without a display | every commit |
| UI component tests | Selection, timeline filter, loupe (pick/reject/stars), template editor, profiles | `i-slint-backend-testing` (`ElementHandle`, no window) | every commit |
| UI end-to-end tests | Core flows against the running app: device appears → select → import → "card can be formatted" | Slint feature `system-testing` + test driver via `SLINT_TEST_SERVER` | every PR |
| Screenshot comparisons | Main screens in light and dark theme, HiDPI | Slint software renderer + reference hashes (ADR 0017) | every PR |
| Accessibility smoke tests | Drive the app from outside via platform accessibility | AT-SPI (dogtail) on Linux, Appium Mac2 on macOS | nightly |
| Hardware tests | Real camera (PTP), Android (MTP), card reader | Hardware-in-the-loop test or manual checklist | before release |

## 9.2 Deliberately injected faults (required scenarios)

| ID | Scenario | Expectation |
|---|---|---|
| T1 | Process is killed in the middle of copying | No `.part` leftovers treated as finished files; resuming skips already-verified files (N2, N4) |
| T2 | Destination fills up (small tmpfs) | Import stops cleanly, nothing partial at the destination, clear message (I1) |
| T3 | Bit error at the destination after writing | Verification catches it, the file is rewritten or reported as an error (N1) |
| T4 | Corrupt or truncated RAW file, broken EXIF data | A single file never aborts the run (N26) |
| T5 | Slow or hanging device | Per-file time limit, UI stays responsive, memory stays bounded |
| T6 | Device is unplugged during the import | Clean abort, resumes after reconnecting |
| T7 | Backup destination missing or disappears | Import keeps running, backup is marked as pending, the card is **not** released (N5) |
| T8 | Same file on both card slots, renamed file, FAT32 timezone change | Content-based recognition correctly handles each case (N3, Q8) |
| T9 | Camera file number overflows (9999 → 0001) | No name conflicts (G7) |
| T10 | Destination is on a different filesystem than the staging area | No "Invalid cross-device link" |

## 9.3 Test data

Large sample files are not stored in the source repository. They live in the separate
`framehive-testdata` repository and are pinned in `testdata.lock` (ADR 0017).

- Memory-card images (FAT32 and exFAT) with a DCIM structure, mounted as a loop device
  or used directly as a folder source.
- Fujifilm X-T2 and X-T5 reference set (F7): RAF in every compression mode, JPEG and HEIF
  pairs, a dual-slot "RAW / JPEG separation" card pair; from raw.pixls.us (CC0) and
  the project owner's own photos (released for the repository).
- Canon EOS 7D reference set (C6): CR2 in RAW, M-RAW and S-RAW, a CR2 + JPG pair, an
  `_MG_` (Adobe RGB) pair, a video with `.THM`; own photos if available, otherwise
  raw.pixls.us.
- DNG reference set (D7): Adobe DNG Converter output (lossless, lossy, linear, without
  preview, with embedded original), a DJI drone DNG, a GoPro GPR; from raw.pixls.us and
  the project owner's own files.
- RAW samples per manufacturer from raw.pixls.us (CC0 license), plus HEIF, videos from
  Sony, GoPro (LRV/GPR), and iPhone.
- Virtual camera via libgphoto2's test driver for PTP; a fake implementation of the
  `DeviceSource` trait for MTP and iOS scenarios.

## 9.4 Architecture rules for testability

1. Logic lives in Rust view models; the Slint UI stays a thin layer. The import plan
   is a pure data structure.
2. Every interactive element has an accessibility label. That's the stable test
   selector (like `getByRole` on the web) and it improves accessibility.
3. Clock, filesystem, and devices are wired in via traits and can be substituted in
   tests.
4. GUI and CLI use the same core, so the core end-to-end tests run via the CLI also
   hold for the GUI.

## 9.5 CI

- Linux (required) and macOS (required) in GitHub Actions; Windows optional.
- On every PR: `cargo fmt`, `clippy -D warnings`, all test levels up through
  screenshot comparisons, `cargo deny`.
- Nightly: accessibility smoke tests, fuzzing the metadata parsers (`cargo-fuzz`),
  performance benchmark against the targets from section 7 (`criterion`).

## 9.6 Library

| ID | Scenario | Expectation |
|---|---|---|
| T11 | XMP round trip with sidecars from digiKam, darktable, and Lightroom | Read, modify, write, and the other program reads everything back correctly; unknown fields are preserved |
| T12 | Migration from a sample `digikam4.db` | Tags, ratings, labels, albums, faces fully carried over; report is accurate |
| T13 | Two machines change the same image | Merge per S8, no lost tags |
| T14 | Root goes offline, images get tagged, root comes back | Queued changes end up in the XMP (S3) |
| T15 | Delete and rebuild the index | Same state as before (5.1) |
| T16 | External change (darktable sets a rating) | Is detected and picked up (S6) |
| T17 | An import is running while a second one is queued and tagging happens in the library | No name conflicts, no blocked UI (J1–J3) |
| T18 | Performance with a synthetic library of 200,000 images | Targets from section 7 |

## 9.7 Risk

Slint's test APIs (`i-slint-backend-testing`, `system-testing`) aren't yet as mature
as Playwright and are partly marked unstable. Validate with a prototype before
committing to them (see section 11).
