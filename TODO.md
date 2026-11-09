# Open work

One line per item, newest at the end. A finished item is deleted. Open questions about a
feature live in its spec, decisions in `docs/adr/`. Add items with `/todo`.

## Open

- [ ] Provide the missing reference files — owner; clean them with `clean.sh` and publish them in `framehive-testdata` as new sets under the Framehive Test Data Licence. Published: one lossless RAF + JPG pair each from the X-T2 (`fujifilm-x-t2-v1`) and X-T5 (`fujifilm-x-t5-v1`), and a Lightroom DNG + JPG pair from the EOS 7D (`canon-eos-7d-v1`). Missing for CORE-001: an uncompressed RAF and a *Classic Chrome* shot from the X-T2; uncompressed and lossy compressed RAFs, a RAF + HIF pair, a *Nostalgic Neg.* shot and a dual-slot "RAW / JPEG separation" pair from the X-T5. Missing for CORE-003: EOS 7D CR2s (RAW, M-RAW, S-RAW), a CR2 + JPG pair, an Adobe RGB `_MG_` pair and an `MVI_` MOV with its THM (CORE-001, CORE-003, ADR 0017) (2026-10-18)
- [ ] Extend the module template in `xtask/templates/module/` once the kernel has the `Module` contract, `kernel::testing::module_conformance` and `ui-kit`: implement `Module`, call the conformance suite, add a Slint view (architecture §4.5) — gated: waits for the GUI milestone, skip it until then (owner decision #68, 2026-11-08) (2026-10-19)
- [ ] Store reference pixel hashes for aarch64-darwin in the LibRaw version test — on macOS it compares only sizes until the hashes are measured on a Mac (PREVIEWS-001, ADR 0028) (2026-11-03)
- [ ] Export a handful of sidecars from digiKam on own photos as XMP fixtures: cleaned (no GPS, serial number or people), under the Framehive Test Data Licence — owner; darktable is done (EOS 350D, EOS 7D, X-T2, X-T5); Lightroom and Capture One samples only if at hand (CORE-010.11, ADR 0039, D26) (2026-11-08)
- [ ] Keep RAW and Remove RAWs in the library: the marker in the index and every member's sidecar, key `R`, the filter, Remove RAWs through a trash port with its failure-path tests, and the summary with the two thumbnail groups (LIBRARY-002.1, .2, .7–.17; LIBRARY-001.15, .16). The XMP field is done (.3–.6). Gated: waits for the library module and the GUI milestone (2026-11-09)

## XMP milestone

Owner decisions D1–D27 of 2026-11-08 (requirements §12, ADR 0039, ADR 0040), in this order.

- [ ] XMP PR 7: the digiKam fixtures (CORE-010.11, D25); needs the owner's fixtures. darktable (CORE-010.9) and the exiftool round trips (CORE-010.8, .10) are done (2026-11-08)
