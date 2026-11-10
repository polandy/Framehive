# Open work

One line per item, newest at the end. A finished item is deleted. Open questions about a
feature live in its spec, decisions in `docs/adr/`. Add items with `/todo`.

## Open

- [ ] Extend the module template in `xtask/templates/module/` once the kernel has the `Module` contract, `kernel::testing::module_conformance` and `ui-kit`: implement `Module`, call the conformance suite, add a Slint view (architecture §4.5) — gated: waits for the GUI milestone, skip it until then (owner decision #68, 2026-11-08) (2026-10-19)
- [ ] Store reference pixel hashes for aarch64-darwin in the LibRaw version test — on macOS it compares only sizes until the hashes are measured on a Mac (PREVIEWS-001, ADR 0028) (2026-11-03)
- [ ] Keep RAW and Remove RAWs in the GUI: key `R`, the filter, the index and the summary with the two thumbnail groups (LIBRARY-002.1, .2, .8, .13, .14; LIBRARY-001.15, .16), on the Remove RAWs model of the `library` module; the CLI commands are done (CLI-018, #109). Gated: waits for the GUI milestone (2026-11-09)
- [ ] After the 0.1.0 release is tagged, remove `release-as`, `skip-changelog` and `bootstrap-sha` from `release-please-config.json`, so the next release is computed from the commits and release-please writes `CHANGELOG.md` (ADR 0045 §4) (2026-11-10)

## XMP milestone

Owner decisions D1–D27 of 2026-11-08 (requirements §12, ADR 0039, ADR 0040), in this order.

