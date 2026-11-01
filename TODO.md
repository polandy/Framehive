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
- [ ] Coverage-guided fuzzing of the EXIF parser (`metadata`) with cargo-fuzz needs an ADR, because it links the native libFuzzer (ADR 0009); until then a deterministic mutation test stands in (CORE-005) (2026-10-31)
- [ ] Compare target names in one Unicode normalization form (NFC or NFD) in conflict resolution: macOS file systems may store names decomposed, so a precomposed name from a card and its decomposed form at the destination are not recognized as the same target; today names are only compared after `str::to_lowercase` (IMPORT-003) (2026-10-31)
- [ ] Guided import PR 2: split `import::ImportRun` into `plan()` and `execute(plan)`, with a plan for a subset of the files as a preview; `Spec-Unchanged:`, amend ADR 0020 if needed — owner decided (2026-11-01)
- [ ] Guided import PR 3: ADR 0023 device detection (poll mounts with std: `/proc/self/mountinfo` on Linux, `/Volumes` on macOS; only drives with a `DCIM` folder; eject through `udisksctl`/`diskutil` processes), spec DEVICES-001, service `crates/services/devices` with the port `DeviceSource`, adapter `crates/adapters/mounts` with its conformance suite — owner decided (2026-11-01)
- [ ] Guided import PR 4: ADR 0024 line-based guided import (no full-screen TUI, no terminal library), ADR 0025 remembered state as a plain-text file in `~/.local/state/framehive` (library folder, template, job codes), spec CLI-002 for the guided import that `framehive` starts without arguments, with a template step offering 3 presets and a card preview, and a job-code prompt only when the template has `{job}`; extend IMPORT-006 with the remembered list of job codes — owner decided (2026-11-01)
- [ ] Write the job code to XMP as IPTC Job Id (`photoshop:TransmissionReference`, N16) with the XMP milestone, and store job codes in profiles (N8) with the YAML configuration (R9, IMPORT-006) — owner decided (2026-11-01)
