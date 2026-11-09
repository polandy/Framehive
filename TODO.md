# Open work

One line per item, newest at the end. A finished item is deleted. Open questions about a
feature live in its spec, decisions in `docs/adr/`. Add items with `/todo`.

## Open

- [ ] Provide sample files from the X-T2, X-T5 and EOS 7D for the reference sets — owner; cleaned and published in `framehive-testdata` (CORE-001, CORE-003, ADR 0017) (2026-10-18)
- [ ] Create the `framehive-testdata` repository and publish the first `.tar.zst` reference set — owner, together with the sample files (ADR 0017, ADR 0027) (2026-11-03)
- [ ] Extend the module template in `xtask/templates/module/` once the kernel has the `Module` contract, `kernel::testing::module_conformance` and `ui-kit`: implement `Module`, call the conformance suite, add a Slint view (architecture §4.5) — gated: waits for the GUI milestone, skip it until then (owner decision #68, 2026-11-08) (2026-10-19)
- [ ] Store reference pixel hashes for aarch64-darwin in the LibRaw version test — on macOS it compares only sizes until the hashes are measured on a Mac (PREVIEWS-001, ADR 0028) (2026-11-03)
- [ ] Export a handful of sidecars from darktable and digiKam on own photos as XMP fixtures: cleaned (no GPS, serial number or people), CC0 — owner; Lightroom and Capture One samples only if at hand (CORE-009.23, ADR 0039, D26) (2026-11-08)

## XMP milestone

Owner decisions D1–D27 of 2026-11-08 (requirements §12, ADR 0039, ADR 0040), in this order.

- [ ] XMP PR 7: the darktable and digiKam fixtures (CORE-010.9, D25); needs the owner's fixtures. The exiftool round trips (CORE-010.8, .10) are done (2026-11-08)
