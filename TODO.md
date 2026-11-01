# Open work

One line per item, newest at the end. A finished item is deleted. Open questions about a
feature live in its spec, decisions in `docs/adr/`. Add items with `/todo`.

## Open

- [ ] Review the proposed ADRs 0004, 0007, 0008, 0012 and 0013 and accept or revise them — the owner decides (2026-10-18)
- [ ] Provide sample files from the X-T2, X-T5 and EOS 7D for the reference sets — owner; cleaned and published in `framehive-testdata` (CORE-001, CORE-003, ADR 0017) (2026-10-18)
- [ ] Create the `framehive-testdata` repository, `testdata.lock` and `cargo xtask testdata fetch` — the fetch needs an HTTP client and BLAKE3 through ADR 0009 (ADR 0017) (2026-10-18)
- [ ] Write `docs/xmp-namespace.md` and choose Framehive's XMP namespace URI — IMMICH-001 already refers to it (2026-10-18)
- [ ] Implement `cargo xtask schemas` and the first JSON Schema in `schemas/` (ADR 0011) (2026-10-18)
- [ ] Check the `Spec-Unchanged:` and `Spec-Only:` trailers against the base branch, and add `cargo xtask spec-report` (ADR 0010) (2026-10-18)
- [ ] Extend the module template in `xtask/templates/module/` once the kernel has the `Module` contract, `kernel::testing::module_conformance` and `ui-kit`: implement `Module`, call the conformance suite, add a Slint view (architecture §4.5) (2026-10-19)
- [ ] Fix the template in LIBRARY-001.12: `{date:%Y%m%d}_{seq_session:4}` lacks `.{ext}` and would be rejected by IMPORT-001 validation — before LIBRARY-001 is accepted (2026-10-31)
- [ ] Write a TUI concept before any graphical UI work, after milestone 1 (CLI import): as user-friendly as possible, e.g. it shows when an SD card is inserted and lets the user select it, then plan, preview and run the import; covers device detection (udisks, macOS DiskArbitration), the TUI library (ADR 0009), and the layer that hosts it (architecture §3) — owner asked (2026-10-31)
- [ ] Coverage-guided fuzzing of the EXIF parser (`metadata`) with cargo-fuzz needs an ADR, because it links the native libFuzzer (ADR 0009); until then a deterministic mutation test stands in (CORE-005) (2026-10-31)
- [ ] Compare target names in one Unicode normalization form (NFC or NFD) in conflict resolution: macOS file systems may store names decomposed, so a precomposed name from a card and its decomposed form at the destination are not recognized as the same target; today names are only compared after `str::to_lowercase` (IMPORT-003) (2026-10-31)
- [ ] Write a spec for job codes: a `{job}` token in name templates and `--job-code` on `framehive import`; remembering the list, prompting at start and writing it to XMP (N16) belong with the TUI concept (R9, IMPORT-001, CLI-001) — owner asked (2026-11-01)
