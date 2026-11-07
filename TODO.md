# Open work

One line per item, newest at the end. A finished item is deleted. Open questions about a
feature live in its spec, decisions in `docs/adr/`. Add items with `/todo`.

## Open

- [ ] Provide sample files from the X-T2, X-T5 and EOS 7D for the reference sets — owner; cleaned and published in `framehive-testdata` (CORE-001, CORE-003, ADR 0017) (2026-10-18)
- [ ] Create the `framehive-testdata` repository and publish the first `.tar.zst` reference set — owner, together with the sample files (ADR 0017, ADR 0027) (2026-11-03)
- [ ] Extend the module template in `xtask/templates/module/` once the kernel has the `Module` contract, `kernel::testing::module_conformance` and `ui-kit`: implement `Module`, call the conformance suite, add a Slint view (architecture §4.5) (2026-10-19)
- [ ] Write the job code to XMP as IPTC Job Id (`photoshop:TransmissionReference`, N16) with the XMP milestone (R9, IMPORT-006) — owner decided (2026-11-01)
- [ ] Make release builds reproducible: `SOURCE_DATE_EPOCH`, `--remap-path-prefix` and a bit-for-bit check in CI, with the first release build (ADR 0007) (2026-11-03)
- [ ] Store reference pixel hashes for aarch64-darwin in the LibRaw version test — on macOS it compares only sizes until the hashes are measured on a Mac (PREVIEWS-001, ADR 0028) (2026-11-03)
- [ ] Add a CORE-006 acceptance criterion for a list entry with no value (`<key> has an entry with no value`) once the spec is split below 10 KB — only an unmarked test covers it (CORE-006, PR #41 review) (2026-11-05)
- [ ] Show `ImportSummary::free_space_unknown` in the CLI or drop the field — the CLI prints the warnings of its own first check, and only tests read the field (IMPORT-012, I1 review) (2026-11-05)
- [ ] Give videos a name template of their own — today they use the import's name template (IMPORT-013, R7, owner decision D2) (2026-11-06)
- [ ] Decide whether `JobQueue::stop` should fail for a job that has just finished but whose end is not yet recorded — it returns `Ok` today, as CORE-007.11 allows (CORE-007, PR #46 review) (2026-11-07)
- [ ] Keep `framehive import-log` from leaving `-wal`/`-shm` files in the state folder — it opens the import history with `SqliteImportHistory::read_only`, which creates them (verified by hand); fix it in the sqlite adapter or revisit ADR 0031; the import dry run avoids it with a read-write connection that never writes (CLI-009, CLI-008.11, ADR 0031) (2026-11-07)
- [ ] Make the older CLI tests that use `set_mode`/`chmod` to make files unreadable fail for every user — they fail when run as root; switch them to failures that hold for any user, as CLI-015.2, CLI-015.3 and CLI-015.9 do (2026-11-07)
- [ ] Report a target whose folder is a file at the destination when the run is planned — the dry run says it would copy, and the run then fails the file with a misleading "<folder> does not exist"; with a video destination inside the import destination, a photo copied earlier in the run can be that file (e.g. template `{number}.RAF/{name}.{ext}`) (IMPORT-003, IMPORT-013) (2026-11-07)
