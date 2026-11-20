# Open work

One line per item, newest at the end. A finished item is deleted. Open questions about a
feature live in its spec, decisions in `docs/dev/adr/`. Add items with `/todo`.

## Open

- [ ] Extend the module template in `xtask/templates/module/` once the kernel has the `Module` contract, `kernel::testing::module_conformance` and `ui-kit`: implement `Module`, call the conformance suite, add a Slint view (architecture §4.5) — gated: waits for the GUI milestone, skip it until then (owner decision #68, 2026-11-08) (2026-10-19)
- [ ] Store reference pixel hashes for aarch64-darwin in the LibRaw version test — on macOS it compares only sizes until the hashes are measured on a Mac (PREVIEWS-001, ADR 0028) (2026-11-03)
- [ ] Keep RAW and Remove RAWs in the GUI: key `R`, the filter, the index and the summary with the two thumbnail groups (LIBRARY-002.1, .2, .8, .13, .14; LIBRARY-001.15, .16), on the Remove RAWs model of the `library` module; the CLI commands are done (CLI-018, #109). Gated: waits for the GUI milestone (2026-11-09)
- [ ] Drive CLI-013 and CLI-014 through the libgphoto2 virtual camera driver in CI (requirements section 9.3); `framehive cameras` with the one virtual camera and without a camera, `camera:` alone, a port that cannot be opened, a model chosen ignoring case, an unknown model, and the import log and the DCIM note (CLI-013.1 with one camera, .2, .4, .5, .7, .10, .12, .14, and the first case of .8) are done through `vusb` (ADR 0034, amendment of 2026-11-19), the other criteria are open (2026-11-15)
- [ ] Optional, with an ADR 0010 amendment: add a level column to `spec-report` and make `check-specs` require a marker under `apps/cli/tests/` for every `CLI-*` criterion (2026-11-15)
