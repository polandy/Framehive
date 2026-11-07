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
- [ ] Store reference pixel hashes for aarch64-darwin in the LibRaw version test — on macOS it compares only sizes until the hashes are measured on a Mac (PREVIEWS-001, ADR 0028) (2026-11-03)
- [ ] Merge the duplicated `unfinished()` helper — it exists in both `crates/services/files/src/copy/target.rs` and `modules/import/import/src/run/copy.rs` (IMPORT-007, PR #36 review) (2026-11-04)
- [ ] Add a CLI-009 acceptance criterion for a missing state folder and mark its test `without_a_state_folder_the_command_stops` — it tests prose-only behavior today (CLI-009, PR #40 review) (2026-11-05)
- [ ] Merge the duplicated `capture_time_text` into one `LocalDateTime` formatter — it exists in both `apps/cli/src/import_log/mod.rs` and `crates/adapters/sqlite/src/text.rs` and both write the ADR 0031 format (ADR 0031, PR #40 review) (2026-11-05)
- [ ] Keep the text format of `framehive import-log` on one line per file when a target or original file name contains a newline — JSON and CSV already escape it (CLI-009, PR #40 review) (2026-11-05)
- [ ] Give `schema_version:` without a value its own message — it says "leave the key out to use the default", but the key is required (CORE-006, PR #41 review) (2026-11-05)
- [ ] Add a CORE-006 acceptance criterion for a list entry with no value (`<key> has an entry with no value`) once the spec is split below 10 KB — only an unmarked test covers it (CORE-006, PR #41 review) (2026-11-05)
- [ ] Write the `config init` example to a unique `.part` name — two overlapping runs share `example.yaml.part` and the second truncates the file the first just linked (CLI-010, PR #41 review) (2026-11-05)
- [ ] Test that `cargo xtask check-schemas` reports a changed and a missing schema file — only the path shape is tested (IMPORT-011, PR #41 review) (2026-11-05)
- [ ] Unit-test the free-space adapter's `f_bavail × f_frsize` conversion and its overflow path by moving the multiply into a small function (ADR 0033, I1 review) (2026-11-05)
- [ ] Show `ImportSummary::free_space_unknown` in the CLI or drop the field — the CLI prints the warnings of its own first check, and only tests read the field (IMPORT-012, I1 review) (2026-11-05)
- [ ] A shortage that only the second free-space check in `ImportRun::start` finds comes after `framehive import` created the folders — rare, but it contradicts "nothing is created" (IMPORT-012, CLI-011, I1 review) (2026-11-05)
- [ ] Give videos a name template of their own — today they use the import's name template (IMPORT-013, R7, owner decision D2) (2026-11-06)
- [ ] Catch a photo target that collides with a video target when the video destination lies inside the import destination — they use separate target resolvers; the standard templates cannot produce it (IMPORT-013, R7+I4 review) (2026-11-06)
- [ ] Count only the photo backups in `FormatReadiness::of` for a run without files — `summary.backups` now holds the video backups too, so the verdict of an empty run can name too many locations (IMPORT-008, R7+I4 review) (2026-11-06)
- [ ] Test the CLI line "the import stopped unexpectedly" — it shows only after a panicking import, which the test harnesses cannot inject today (`apps/cli/src/guided/card.rs`, `apps/cli/src/import_command/run.rs`; IMPORT-015, PR #46 review) (2026-11-07)
- [ ] Decide whether `JobQueue::stop` should fail for a job that has just finished but whose end is not yet recorded — it returns `Ok` today, as CORE-007.11 allows (CORE-007, PR #46 review) (2026-11-07)
- [ ] Keep `framehive import-log` from leaving `-wal`/`-shm` files in the state folder — it opens the import history with `SqliteImportHistory::read_only`, which creates them (verified by hand); fix it in the sqlite adapter or revisit ADR 0031; the import dry run avoids it with a read-write connection that never writes (CLI-009, CLI-008.11, ADR 0031) (2026-11-07)
- [ ] Make the older CLI tests that use `set_mode`/`chmod` to make files unreadable fail for every user — they fail when run as root; switch them to failures that hold for any user, as CLI-015.2, CLI-015.3 and CLI-015.9 do (2026-11-07)
