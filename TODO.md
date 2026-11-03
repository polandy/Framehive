# Open work

One line per item, newest at the end. A finished item is deleted. Open questions about a
feature live in its spec, decisions in `docs/adr/`. Add items with `/todo`.

## Open

- [ ] Provide sample files from the X-T2, X-T5 and EOS 7D for the reference sets — owner; cleaned and published in `framehive-testdata` (CORE-001, CORE-003, ADR 0017) (2026-10-18)
- [ ] Create the `framehive-testdata` repository and publish the first `.tar.zst` reference set — owner, together with the sample files (ADR 0017, ADR 0027) (2026-11-03)
- [ ] Write `docs/xmp-namespace.md`: the fields of Framehive's XMP namespace `https://polandy.github.io/framehive/ns/1.0/` (prefix `framehive`, ADR 0013) — IMMICH-001 already refers to it (2026-10-18)
- [ ] Implement `cargo xtask schemas` and the first JSON Schema in `schemas/` (ADR 0011) (2026-10-18)
- [ ] Extend the module template in `xtask/templates/module/` once the kernel has the `Module` contract, `kernel::testing::module_conformance` and `ui-kit`: implement `Module`, call the conformance suite, add a Slint view (architecture §4.5) (2026-10-19)
- [ ] Coverage-guided fuzzing of the EXIF parser (`metadata`) with cargo-fuzz needs an ADR, because it links the native libFuzzer (ADR 0009); until then a deterministic mutation test stands in (CORE-005) (2026-10-31)
- [ ] Write the job code to XMP as IPTC Job Id (`photoshop:TransmissionReference`, N16) with the XMP milestone, and store job codes in profiles (N8) with the YAML configuration (R9, IMPORT-006) — owner decided (2026-11-01)
- [ ] Add the `cargo public-api` snapshot check for `kernel` and the services (architecture §7, ADR 0004) (2026-11-03)
- [ ] Make release builds reproducible: `SOURCE_DATE_EPOCH`, `--remap-path-prefix` and a bit-for-bit check in CI, with the first release build (ADR 0007) (2026-11-03)
- [ ] Build the LibRaw prototype (roadmap phase 0, ADR 0012): pin LibRaw in the flake and decode X-T5 lossy compressed RAF, 7D M-RAW/S-RAW, GoPro GPR and JPEG XL compressed DNG; if lossy RAF fails, a new ADR supersedes 0012 (2026-11-03)
- [ ] Cache `.testdata/` in CI keyed by the hash of `testdata.lock` and run `cargo xtask testdata fetch` there, with the first test that reads test data (ADR 0017) (2026-11-03)
