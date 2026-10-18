# Framehive – Rules for AI agents

Framehive is a modular photo library (Linux + macOS) written in Rust with a Slint UI.
This project is developed by AI agents. Humans must still be able to read and follow
every line. Quality cannot depend on human review, so every rule is enforced by
`cargo xtask ci`.

This file is loaded in full by every session and every subagent, so it holds rules and
pointers only. **Don't grow it**: move detail into `docs/` and keep a pointer here.

## Commands

Run them inside `nix develop`, or prefix them with `nix develop -c`.

- **Before finishing:** `cargo xtask ci`. It mirrors CI, prints one line per step and the
  failing step in full.
- While iterating: `cargo nextest run -p <crate> [test name]`, and single checks with
  `cargo xtask check-files|check-architecture|check-dependencies|check-specs`.
- Open work: `TODO.md`. `/todo` adds an item, `/next` picks and plans a task, `/status` and
  `/review` check the state and a diff. Subagents: `framehive-implementer` (one spec),
  `framehive-reviewer` (one change).

## Read first

| Topic | Document |
|---|---|
| What we build | `docs/requirements/` (one file per section) |
| Exact behavior of each feature | `specs/` (rules in `specs/README.md`) |
| How it is structured | `docs/architecture/` (one file per section), especially §3 layers, §4 modules, §8 standards, §10 glossary |
| Why decisions were made | `docs/adr/` |
| A specific crate | that crate's `README.md` |

## Reading budget

Every tool result is re-read, and paid for, on each later turn.

- **Open only the section a change touches.** Requirements, architecture and specs are
  one file per section or feature; start from the `README.md` index, never read the set.
- **Grep before you read**, and read a large file by offset.
- **Bundle independent commands into one call.** Never repeat a check whose answer
  cannot have changed.
- **Delegate by threshold:** fact-finding reads over ~10 KB, full test or CI runs and
  repository-wide rename sweeps go to a subagent (`Explore` to find, `model: "sonnet"`
  to run) that reports in a few lines. Judgement and spec writing stay with you.
- **Edit, don't rewrite:** change files with targeted edits, never by writing them out
  whole. An analysis you repeat becomes a script or an `xtask` command.
- **One task per session**, under ~150k tokens of context. Review and clean-up run in a
  fresh session. A subagent past ~120k tokens stops and returns a hand-off.

## Non-negotiable rules

1. **Respect the layers** (architecture §3). A module never depends on another
   module. Only `apps/*` know concrete adapters. Check with
   `cargo xtask check-architecture`.
2. **No I/O in domain logic.** Use the ports for time (`Clock`), IDs, randomness,
   files, devices and the database.
3. **Follow the standards exactly** (XMP, MWG, IPTC, EXIF; architecture §8).
   Never drop unknown XMP fields. If a mapping is not defined by a standard, write an
   ADR before implementing it.
4. **Tests are deterministic.** No `sleep`, no wall clock, no network, no real home
   directory, fixed seeds. Never add retries. Never skip or ignore a failing test. Fix
   the cause.
5. **Builds are reproducible.** Use the pinned toolchain and the Nix shell. Run
   `--locked`. Never edit `Cargo.lock` by hand.
6. **Write an ADR before changing architecture.** That covers new crates outside the
   module template, new kernel events or commands, new dependencies with native code,
   and changed file formats.
7. **Keep dependencies minimal** (ADR 0009). Prefer std or small, well-tested own code.
   Use a library only for large, deep or security-critical problems, and only from
   established products. Every direct dependency must be in `dependencies.yaml` with a
   rationale, and it is wrapped in exactly one adapter crate. Native libraries need an ADR.
8. **Use names from the glossary** (architecture §10). Don't introduce synonyms.
9. **Specification and implementation stay in sync** (ADR 0010). Every feature has a
   spec in `specs/`. A spec change changes tests and code in the same commit, and vice
   versa. Each acceptance criterion has at least one test marked `/// spec: <ID>.<n>`.
   Check with `cargo xtask check-specs`.
10. **Configuration is YAML** (ADR 0011). Use self-explanatory full-word keys, explicit
    units, strict validation, `schema_version` and a JSON Schema in `schemas/`.
11. **Everything in the repository is English.** That covers code, comments, docs,
    specs, ADRs, UI strings in mockups, commit messages, PR titles and descriptions,
    and issue text. It applies even when you talk to the user in another language.

## Code style

- All repository content is in English (rule 11).
- Write small units. Each function does one thing, and its name says what. Comments
  explain *why*, not *what*.
- Prefer clarity over cleverness. Avoid macros and generic gymnastics unless they
  clearly reduce complexity.
- Libraries: use `thiserror` error types, no `unwrap`/`expect` outside tests, and no
  `println!` (use `tracing`).
- Every public item has a doc comment. Doc examples must compile.
- No `unsafe` outside the FFI adapter crates. There, every block has a `// SAFETY:`
  comment.

## Workflow

1. Understand the task. Read the relevant spec, README, ADRs and glossary terms.
2. Spec first: write or update the spec in `specs/` for any change in behavior.
3. Write or extend the tests, at the lowest layer that can express the behavior.
4. Implement the smallest change that makes the tests pass. Keep each change focused on
   one concern.
5. Run `cargo xtask ci` (the same as CI). It must be green. It prints one line per
   step and the full output only for a failing step.
6. Update docs: the spec status, the crate README, `CHANGELOG.md` for user-visible
   changes, and the ADRs and requirements if they are affected.
7. Commit using Conventional Commits, in English. The body explains why. Use the
   `Spec-Unchanged:` or `Spec-Only:` trailer when ADR 0010 asks for it. No `Co-Authored-By`
   or "Generated with" lines, in commits or PR descriptions (`.claude/settings.json`
   turns them off for Claude Code).

## Adding things

- New module: `cargo xtask new-module <id>`, then follow architecture §4.5.
- New adapter: `cargo xtask new-adapter <port> <name>`. It must pass the port's
  conformance suite.
- New dependency: first check whether std or own code is enough. If not, add an entry to
  `dependencies.yaml` that meets the criteria in ADR 0009. It must pass
  `cargo xtask check-dependencies` and `cargo deny`.
- New feature: add `specs/<module>/<ID>-<name>.md` from the template in `specs/README.md`.
- New config option: extend the typed config struct, regenerate the schema with
  `cargo xtask schemas`, and document the key in `framehive config init`.
