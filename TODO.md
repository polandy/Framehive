# Open work

One line per item, newest at the end. A finished item is deleted. Open questions about a
feature live in its spec, decisions in `docs/adr/`. Add items with `/todo`.

## Open

- [ ] Extend the module template in `xtask/templates/module/` once the kernel has the `Module` contract, `kernel::testing::module_conformance` and `ui-kit`: implement `Module`, call the conformance suite, add a Slint view (architecture §4.5) — gated: waits for the GUI milestone, skip it until then (owner decision #68, 2026-11-08) (2026-10-19)
- [ ] Store reference pixel hashes for aarch64-darwin in the LibRaw version test — on macOS it compares only sizes until the hashes are measured on a Mac (PREVIEWS-001, ADR 0028) (2026-11-03)
- [ ] Keep RAW and Remove RAWs in the library: the marker in the index and every member's sidecar, key `R`, the filter, Remove RAWs through a trash port with its failure-path tests, and the summary with the two thumbnail groups (LIBRARY-002.1, .2, .7–.17; LIBRARY-001.15, .16). The XMP field is done (.3–.6). Gated: waits for the library module and the GUI milestone (2026-11-09)

## XMP milestone

Owner decisions D1–D27 of 2026-11-08 (requirements §12, ADR 0039, ADR 0040), in this order.

