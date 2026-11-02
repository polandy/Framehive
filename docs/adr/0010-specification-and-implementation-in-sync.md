# 0010 – Every feature is specified, specification and implementation stay in sync

- Status: accepted
- Date: 2026-10-17

## Context

The project is developed by AI agents. Humans should be able to read at any time what
the software does, without reading the code. Agents need an unambiguous, verifiable
specification. Experience shows that specifications go stale quickly unless they are
technically coupled to the code.

## Decision

### 1. Every feature has a specification

- Every feature is specified in `specs/<module>/<ID>-<name>.md` (template and rules:
  `specs/README.md`). Cross-cutting features (kernel, services) live under
  `specs/core/`.
- A specification has a stable ID (`IMPORT-001`), a status, its origin (requirement
  IDs from `docs/requirements/`, e.g. R4, J3), a description of the behavior and
  **numbered acceptance criteria** (`IMPORT-001.3`) in Given / When / Then form.
- `docs/requirements/` remains the catalog (what, why, priority); the specification is
  the binding, detailed description of the behavior. Mockups are illustrative, not
  binding.

### 2. Coupling to tests

- Every acceptance criterion is backed by at least one test. The test names the
  criterion in a doc line:

  ```rust
  /// spec: IMPORT-001.3
  #[test]
  fn raw_and_jpeg_of_same_shot_share_sequence_number() { … }
  ```

  Deliberately without a macro library (ADR 0009); `xtask` reads these lines.
- Every test at the module or system level (use cases, ViewModels, CLI, GUI) belongs
  to an acceptance criterion. Pure implementation details (unit tests of internal
  functions) need no such link.

### 3. Synchronization in both directions

| Change | Required in the same commit |
|---|---|
| Specification changed (behavior, criterion added/changed/removed) | Tests and implementation updated |
| Implementation changes observable behavior | Specification updated, related tests updated |
| New feature | Specification first (status `accepted`), then tests, then code |
| Feature removed | Specification set to `removed` with a rationale, tests and code removed |

Specification, tests and code never drift apart: `main` contains, at every point in
time, a specification that describes exactly the implemented behavior. Planned
features that are not yet implemented have status `accepted` and are unreachable in
the code.

### 4. Automated checks (`cargo xtask check-specs`, part of `cargo xtask ci`)

- Every specification has valid required fields; IDs are unique; origin IDs exist in
  `docs/requirements/`.
- Every criterion of a specification with status `implemented` has at least one test;
  every `spec:` reference in a test points to an existing criterion.
- No test references a specification with status `draft` or `removed` (otherwise the
  status would be wrong).
- Diff check: if a commit changes tests with a `spec:` reference, the affected
  specification must change in the same commit, or the commit message contains
  `Spec-Unchanged: <reason>` (e.g. a pure test refactor). If a commit changes a
  specification with status `implemented`, the related tests or code must change too,
  or the commit message contains `Spec-Only: <reason>` (e.g. pure rewording).
- `cargo xtask spec-report` produces an overview: requirement → specification →
  criteria → tests, with status. It is produced as a CI artifact.

**Amendment (2026-11-02): the diff check in detail.** `cargo xtask check-spec-trailers`,
a step of `cargo xtask ci`, implements the diff check as follows.

- *Which commits.* In a pull request, `origin/<base>..HEAD` with the base branch from
  `GITHUB_BASE_REF`; each commit of the pull request is judged on its own. On `main`,
  only the head commit (`HEAD^..HEAD`): the squash commit of a merged pull request is
  judged as a whole. Locally, `main..HEAD`. Older history is never checked.
- *Merge commits are skipped.* On a branch they only bring in `main`, whose commits are
  checked on `main`; the squash merge removes them anyway. The synthetic merge commit
  GitHub checks out for a pull request is skipped for the same reason.
- *Trailers.* A line `Spec-Unchanged: <reason>` or `Spec-Only: <reason>` with a
  non-empty reason counts anywhere in the message, because a squash commit collects the
  messages of all commits of its pull request.
- *Rule A (`Spec-Unchanged:`).* A changed `.rs` file that contains `/// spec:` markers
  for a specification, before or after the commit, requires that specification's file
  to change in the same commit.
- *Rule B (`Spec-Only:`).* A changed specification whose status is `implemented` after
  the commit (before it, if the file was deleted) requires a changed `.rs` file in a
  crate that holds markers for it. If no crate holds markers for it, any changed `.rs`
  file counts.
- *A violation fails CI*, like every other check.
- *Missing base.* Without a local `main`, a local run skips the check with a notice. In
  CI (`CI=true`) a base that cannot be resolved, as in a shallow clone, is an error, so
  the check never passes silently; CI therefore checks out the full history.

Example specification path: `specs/import/IMPORT-002-sequence-numbers.md`.

## Alternatives

- *Executable Cucumber/Gherkin*: an extra dependency and a second test language; the
  ID-based coupling achieves the same with what we already have.
- *`docs/requirements/` only*: too coarse for unambiguous behavior and not coupled to
  tests.

## Consequences

- Every behavior is readable by humans in prose and backed by tests.
- More effort per change; in return, the specification never goes stale.
- The diff check is a heuristic: it forces specification and tests to be considered
  together, but it cannot check whether the prose is actually correct. That remains
  the agent's responsibility and is required by the workflow (`CLAUDE.md`).
