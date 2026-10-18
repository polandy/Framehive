# Open work

One line per item, newest at the end. A finished item is deleted. Open questions about a
feature live in its spec, decisions in `docs/adr/`. Add items with `/todo`.

## Open

- [ ] Review the proposed ADRs 0004–0008, 0012 and 0013 and accept or revise them — the owner decides (2026-10-18)
- [ ] Provide sample files from the X-T2, X-T5 and EOS 7D for the reference sets — owner; cleaned and published in `framehive-testdata` (CORE-001, CORE-003, ADR 0017) (2026-10-18)
- [ ] Create the `framehive-testdata` repository, `testdata.lock` and `cargo xtask testdata fetch` — the fetch needs an HTTP client and BLAKE3 through ADR 0009 (ADR 0017) (2026-10-18)
- [ ] Write `docs/xmp-namespace.md` and choose Framehive's XMP namespace URI — IMMICH-001 already refers to it (2026-10-18)
- [ ] Implement `cargo xtask new-module` and `cargo xtask new-adapter` — named in CLAUDE.md "Adding things", architecture §4.5 (2026-10-18)
- [ ] Implement `cargo xtask schemas` and the first JSON Schema in `schemas/` (ADR 0011) (2026-10-18)
- [ ] Check the `Spec-Unchanged:` and `Spec-Only:` trailers against the base branch, and add `cargo xtask spec-report` (ADR 0010) (2026-10-18)
