# 0042 – Keep RAW marker in the Framehive XMP namespace

- Status: accepted
- Date: 2026-11-09

## Context

The owner keeps the RAW for a few pictures and only the JPEG for most (LIBRARY-002,
B29, B30). A Keep RAW marker per asset, independent of rating, pick state and color
label, records which RAWs stay; Remove RAWs moves the other RAWs to the trash. The same
rule was agreed for honeycomb (its plan, PRs #26 and #27).

Files and XMP are the source of truth (ADR 0003), so the marker must be in the sidecars,
and the index must be rebuildable from them. No standard (XMP, IPTC, MWG, EXIF) and no
other program has a field for it. CLAUDE.md rule 3 asks for an ADR before such a mapping
is implemented. A keyboard shortcut is needed too (N14).

## Decision

1. **The marker is `framehive:KeepRaw`**, a simple XMP Boolean in Framehive's namespace
   (ADR 0041, `docs/xmp-namespace.md`), written to the sidecar of every member like the
   rating.
2. **Marked writes `True`; unmarked removes the property.** A sidecar of an unmarked
   asset thus gains nothing, and other programs see no new field on most assets.
3. **Reading is lenient, writing is strict.** `True` in any case counts as marked; a
   missing property, `False` and any other text count as unmarked. A struct or an
   array under that name is another program's data and is never overwritten.
4. **Key `R`.** It is free in the library key map (`P`, `X`, `1`–`9`, and `C`, `G`, `T`,
   `Z` in the mockups). Lightroom's `R` is Crop, which Framehive does not have, and the
   letter matches the word RAW.

## Alternatives

- **Reuse a standard field**, such as a color label or a keyword `Keep RAW`: other
  programs show and change it, and it would take a label or a tag from the user. The
  owner asked for a marker of its own. Rejected.
- **Store the marker only in the library database:** lost on an index rebuild, against
  ADR 0003. Rejected.
- **Write `False` when unmarked:** every sidecar Framehive touches would gain a field
  with no information. Rejected.
- **Key `K` (keep):** reads as pick ("keep") in other programs, which Framehive's pick
  state already covers. Rejected.
