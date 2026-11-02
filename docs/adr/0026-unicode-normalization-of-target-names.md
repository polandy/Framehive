# 0026 – Unicode normalization of target names

- Status: accepted
- Date: 2026-11-01

## Context

The conflict resolution of the import (IMPORT-003) decides whether a target name is
already taken, at the destination or by an earlier file of the plan. It ignores case,
and today compares names after `str::to_lowercase` only.

Unicode allows one name to be written in more than one form. `é` is either the single
code point U+00E9 (precomposed, NFC) or `e` followed by U+0301 (decomposed, NFD). Memory
cards and Linux keep the form they are given, and so do most source names. macOS file
systems and SMB shares served from them may store or return names decomposed. A
precomposed name from a card and the decomposed name of the same file at the destination
are then different strings:

- a second import of the same card copies the photo again instead of marking it
  "already imported";
- on a file system that treats both forms as one name, two different photos claim the
  same target.

The Unicode Standard (§3.13, "canonical caseless match") defines how two strings are
compared without regard to case and form: `NFD(toCasefold(NFD(X)))`. The standard
library has no normalization. Own code would need the canonical decomposition and
composition tables of the Unicode Character Database, the combining classes and the
Hangul algorithm: generated data plus an algorithm well beyond the ~300 lines ADR 0009
allows for own code, and easy to get subtly wrong.

## Decision

1. **Target names are compared by their canonical caseless key**,
   `NFD(lowercase(NFD(name)))`, following Unicode §3.13. `str::to_lowercase` stands in
   for full case folding, as it does today. The key is used only for comparing; names
   are written and reported as they are rendered or found at the destination.
2. **`unicode-normalization` gives the normalization forms.** It is used only in the
   module crate `import`, in the one function that computes the key.
3. **No port.** Normalization is a pure function without I/O. Like `thiserror`, the
   crate is allowed in the module layer, but only in `import`. A switch still changes
   one crate.
4. **Scope:** `default-features = false`, no features. The crate then needs only
   `alloc`, and its one dependency is `tinyvec`.

| Criterion (ADR 0009 §3) | `unicode-normalization` |
|---|---|
| Origin | The unicode-rs organization, which also maintains `unicode-width` and `unicode-segmentation` |
| Adoption | The usual normalization crate in Rust, used by rustc to normalize identifiers; over 600 million downloads |
| Maintenance | Regular releases that follow new Unicode versions, the last in October 2026 |
| Stability | **Version 0.1, before 1.0.** Exception, see below |
| License | (MIT OR Apache-2.0) AND Unicode-3.0, all allowed in `deny.toml` |
| Scope | Pure Rust, no native code, one small dependency (`tinyvec`) |

**The exception for stability.** ADR 0009 asks for version ≥ 1.0 or a de-facto standard
with a stable API. `unicode-normalization` is the de-facto standard for normalization in
Rust, and its API (`nfc()`, `nfd()` on `str`) has stayed the same across its 0.1
releases since 2015. A breaking release would change one function in `import`.

## Alternatives

- **`icu_normalizer`** (ICU4X, Unicode Consortium): version 2, so it meets the stability
  criterion, but it pulls in about 20 crates, including data and proc-macro crates, for
  one comparison.
- **Own code:** tables generated from the Unicode Character Database, combining
  classes, composition exclusions and Hangul; see the context.
- **Normalizing only common Latin letters:** small, but not the standard (CLAUDE.md
  rule 3), and wrong for every other script.
- **Normalizing written names to NFC:** changes which names the import writes, a
  behaviour change beyond the comparison; not part of this decision.

## Consequences

- `dependencies.yaml` gains `unicode-normalization`, allowed only in `import`.
- IMPORT-003 states that two target names that differ only in case or normalization
  form are the same target.
- When `unicode-normalization` reaches 1.0, the exception ends; the upgrade is a normal
  dependency update.
