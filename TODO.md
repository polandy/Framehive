# Open work

One line per item, newest at the end. A finished item is deleted. Open questions about a
feature live in its spec, decisions in `docs/adr/`. Add items with `/todo`.

## Open

- [ ] Provide sample files from the X-T2, X-T5 and EOS 7D for the reference sets — owner; cleaned and published in `framehive-testdata` (CORE-001, CORE-003, ADR 0017) (2026-10-18)
- [ ] Create the `framehive-testdata` repository and publish the first `.tar.zst` reference set — owner, together with the sample files (ADR 0017, ADR 0027) (2026-11-03)
- [ ] Write `docs/xmp-namespace.md`: the fields of Framehive's XMP namespace `https://polandy.github.io/framehive/ns/1.0/` (prefix `framehive`, ADR 0013) — IMMICH-001 already refers to it (2026-10-18)
- [ ] Extend the module template in `xtask/templates/module/` once the kernel has the `Module` contract, `kernel::testing::module_conformance` and `ui-kit`: implement `Module`, call the conformance suite, add a Slint view (architecture §4.5) (2026-10-19)
- [ ] Coverage-guided fuzzing of the EXIF parser (`metadata`) with cargo-fuzz needs an ADR, because it links the native libFuzzer (ADR 0009); until then a deterministic mutation test stands in (CORE-005) (2026-10-31)
- [ ] Write the job code to XMP as IPTC Job Id (`photoshop:TransmissionReference`, N16) with the XMP milestone (R9, IMPORT-006) — owner decided (2026-11-01)
- [ ] Add the `cargo public-api` snapshot check for `kernel` and the services (architecture §7, ADR 0004) (2026-11-03)
- [ ] Make release builds reproducible: `SOURCE_DATE_EPOCH`, `--remap-path-prefix` and a bit-for-bit check in CI, with the first release build (ADR 0007) (2026-11-03)
- [ ] Test or drop the JPEG XL DNG example for `UnsupportedFormat` in PREVIEWS-001 — no CC0 sample yet, and LibRaw may report a data error that the adapter maps to `Corrupt` (ADR 0028, PR #34 review) (2026-11-03)
- [ ] Store reference pixel hashes for aarch64-darwin in the LibRaw version test — on macOS it compares only sizes until the hashes are measured on a Mac (PREVIEWS-001, ADR 0028) (2026-11-03)
- [ ] Merge the duplicated `unfinished()` helper — it exists in both `crates/services/files/src/copy/target.rs` and `modules/import/import/src/run/copy.rs` (IMPORT-007, PR #36 review) (2026-11-04)
- [ ] Add an index on `copies.run_id` at the next import history schema change — the import log reads copies by run; not worth its own migration now (IMPORT-010, PR import-log review) (2026-11-05)
- [ ] Add a CLI-009 acceptance criterion for a missing state folder and mark its test `without_a_state_folder_the_command_stops` — it tests prose-only behavior today (CLI-009, PR #40 review) (2026-11-05)
- [ ] Merge the duplicated `capture_time_text` into one `LocalDateTime` formatter — it exists in both `apps/cli/src/import_log/mod.rs` and `crates/adapters/sqlite/src/text.rs` and both write the ADR 0031 format (ADR 0031, PR #40 review) (2026-11-05)
- [ ] Keep the text format of `framehive import-log` on one line per file when a target or original file name contains a newline — JSON and CSV already escape it (CLI-009, PR #40 review) (2026-11-05)
