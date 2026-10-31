# 0021 – Own EXIF reader for the capture time

- Status: accepted
- Date: 2026-10-31
- Amends: ADR 0020, decision 4, in where `LocalDateTime` lives

## Context

The import names files by their capture time (IMPORT-001). Milestone 1 needs it from
the reference cameras' files: JPEG, Fujifilm RAF (CORE-001), Canon CR2 (CORE-003) and
DNG (CORE-004). CORE-003 and CORE-004 already state that our own TIFF/EXIF reader in the
metadata service reads CR2 and DNG, and ADR 0009 lists "EXIF reading of the needed
fields" as code we write ourselves, to be decided with the metadata ADR. This is that
decision, limited to what the import needs now.

EXIF (CIPA DC-008) is TIFF structure: a byte-order mark, IFDs of 12-byte entries, and
pointers to sub-IFDs. The capture time is `DateTimeOriginal` (tag `0x9003`) in the EXIF
IFD, with `SubSecTimeOriginal` (`0x9291`) and `OffsetTimeOriginal` (`0x9011`).

The architecture overview names an `adapter-exif`. Reading EXIF needs no external
library and no I/O beyond reading bytes, so there is nothing for an adapter to wrap.

## Decision

1. **Our own reader, in `metadata`.** The metadata service reads the capture time
   (CORE-005) with its own code, in pure Rust and without `unsafe`. There is no
   `adapter-exif`.
2. **Containers.**

   | Container | How the EXIF is found |
   |---|---|
   | JPEG (and MPO) | The first `APP1` segment that starts with `Exif\0\0`, before the image data |
   | TIFF-based files | The file itself: byte order `II` or `MM`, magic 42, or the ORF and RW2 variants of the magic. Covers DNG, CR2, NEF, NRW, ARW, SR2, PEF, ORF, RW2, 3FR, DCR, FFF, IIQ, MEF, MOS, SRW and TIFF |
   | RAF | The embedded JPEG, whose offset and length are in the RAF header (big-endian, at bytes 84 and 88), then as JPEG |

   Other formats (CR3, HEIF, PNG, AVIF, JPEG XL, videos and the remaining RAWs) give no
   capture time yet and fall back to the modification time. Each is a later spec.
3. **Reading through the files port.** The reader asks `files::FileSystem` (ADR 0020)
   for byte ranges at offsets, and never reads a whole RAW file. The parsing itself is
   a pure function of those bytes, so it is tested without a file system.
4. **Defensive parsing.** Every offset and length is checked against the file size,
   every IFD is visited at most once (no loops through crafted pointers), and the total
   number of bytes and IFDs read per file is capped. A damaged file gives no capture
   time and a reason, never a panic. The parser is fuzzed (ADR 0009); the fuzzing tool
   is chosen when the code is written.
5. **Test files are built by the tests.** Unit tests assemble JPEG, TIFF and RAF bytes
   in code, in both byte orders. No binary file enters the repository (ADR 0017); the
   reference camera files of CORE-001 and CORE-003 come from `framehive-testdata`.
6. **`LocalDateTime` moves to `domain`**, with `from_timestamp`, next to a new
   `CaptureTime` (date and time, optional sub-seconds, optional UTC offset). Both
   `metadata` and `naming` use them, and neither has to depend on the other for a value
   type. This replaces the row for `naming` in ADR 0020, decision 4.

## Alternatives

- **`kamadak-exif` or `nom-exif`:** they would read the tags, but neither has the
  origin ADR 0009 §3 asks for (an organization or several active maintainers), and the
  container handling for RAF and the RAW variants would still be ours.
- **`exiftool` as an external program:** the most complete reader, but Perl at run
  time, a process per file, and a dependency for users; we use it only as a test
  oracle (architecture §8).
- **An `adapter-exif` around our own reader:** an adapter wraps an external library or
  device; here it would add a crate and a port without anything to swap.

## Consequences

- About 400–600 lines of our own parser code, with unit, property and fuzz tests.
- The same reader is the base for CORE-001, CORE-003 and CORE-004, which add fields
  and maker notes later.
- Architecture §2 and §12 drop `adapter-exif`; the glossary gains `CaptureTime` and
  `LocalDateTime`.
- Formats without EXIF support are dated by their modification time and marked in the
  import plan until their spec adds a reader.
