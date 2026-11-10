# Open work

One line per item, newest at the end. A finished item is deleted. Open questions about a
feature live in its spec, decisions in `docs/adr/`. Add items with `/todo`.

## Open

- [ ] Extend the module template in `xtask/templates/module/` once the kernel has the `Module` contract, `kernel::testing::module_conformance` and `ui-kit`: implement `Module`, call the conformance suite, add a Slint view (architecture §4.5) — gated: waits for the GUI milestone, skip it until then (owner decision #68, 2026-11-08) (2026-10-19)
- [ ] Store reference pixel hashes for aarch64-darwin in the LibRaw version test — on macOS it compares only sizes until the hashes are measured on a Mac (PREVIEWS-001, ADR 0028) (2026-11-03)
- [ ] Keep RAW and Remove RAWs in the library, owner decision A (#109): PR 2 the `library` module with the LIBRARY-001 grouping, the Remove RAWs model and its failure paths (LIBRARY-002.9–.12, .15–.17) and the marker in every member's sidecar (.7); PR 3 the CLI `framehive library keep-raw <files…> [--off]` and `framehive library remove-raws <folder> [--dry-run]`. Done: the XMP field (.3–.6) and the `Trash` port with its adapters (ADR 0043). Key `R`, the filter, the index and the thumbnail summary (.1, .2, .8, .13, .14; LIBRARY-001.15, .16) wait for the GUI milestone (2026-11-09)

## XMP milestone

Owner decisions D1–D27 of 2026-11-08 (requirements §12, ADR 0039, ADR 0040), in this order.

