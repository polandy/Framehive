# 8. Standards

Part of the [architecture](README.md).

We implement standards exactly and are stricter there than most tools. We read
deviations from other programs tolerantly, but write only in a standard-compliant
way.

| Area | Standard | Implementation |
|---|---|---|
| XMP | ISO 16684-1 (XMP Data Model), Adobe XMP Specification Part 3 (sidecars) | Sidecar naming per spec (`image.xmp`), optionally `image.ARW.xmp` for darktable/digiKam; unknown namespaces and fields are preserved unchanged |
| Metadata reconciliation | MWG Guidelines for Handling Image Metadata 2.0 | Priority EXIF/IPTC/XMP on read, synchronization on write, rating (`xmp:Rating`), regions (`mwg-rs`), hierarchical keywords (`mwg-kw`) |
| IPTC | IPTC Photo Metadata Standard (Core + Extension) | Title, description, creator, copyright, locations, people |
| Hierarchical tags | MWG `mwg-kw:Keywords` + de facto `lr:hierarchicalSubject` + flat `dc:subject` | Write all three (compatibility with Lightroom, darktable, digiKam), read `digiKam:TagsList` |
| EXIF | CIPA DC-008 (Exif 3.0), DCF (CIPA DC-009) | Read only; time zones via `OffsetTime*` |
| TIFF, DNG | TIFF 6.0, Adobe DNG Specification 1.7 | Own reader for CR2 and DNG structure; XMP embedded into DNG only as an option (ADR 0015) |
| Color labels | `xmp:Label` (text, Lightroom-compatible) | Color ↔ text mapping configurable, default matches Lightroom |
| Pick/reject | No official standard | Mapping documented in an ADR (e.g. reject = `xmp:Rating -1`, as in Lightroom and darktable) |
| Geodata | EXIF GPS, GPX 1.1 | Geotagging from GPX, written to `exif:GPS*` in XMP |
| Directories | XDG Base Directory, freedesktop.org Trash Spec; macOS File System Programming Guide | Configuration, cache, data, trash |
| Checksums | BLAKE3 | In the log with algorithm prefix (`blake3:…`) |
| Versioning | SemVer 2.0, Conventional Commits 1.0, Keep a Changelog 1.1 | Releases and commit history |
| Licenses | SPDX, REUSE 3 | License declaration per file, `reuse lint` in CI |
| Software bill of materials | CycloneDX | SBOM per release (`cargo cyclonedx`) |

For every standard there are conformance tests with reference files (including
examples from the specifications) and round-trip tests against exiftool, digiKam and
darktable (T11).
