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
  IDs from `docs/dev/requirements/`, e.g. R4, J3), a description of the behavior and
  **numbered acceptance criteria** (`IMPORT-001.3`) in Given / When / Then form.
- `docs/dev/requirements/` remains the catalog (what, why, priority); the specification is
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
  `docs/dev/requirements/`.
- Every criterion of a specification with status `implemented` has at least one test;
  every `spec:` reference in a test points to an existing criterion. A reference counts
  only on a `#[test]` function, so a marker on a helper or a stray line fails the check
  (amendment 2026-11-19).
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
- *The future squash commit.* The squash commit is built from the pull request title
  and description (`docs/dev/pull-requests.md` §3), so a pull request run also judges one
  composed commit: its message is the title, a blank line and the description; its
  changed files are those of `merge-base(origin/<base>, HEAD)..HEAD`; rules A and B
  apply as to the squash commit on `main`. The workflow passes the title and description
  as `PULL_REQUEST_TITLE` and `PULL_REQUEST_BODY`. In CI (`CI=true`) a pull request run
  without `PULL_REQUEST_TITLE` is an error; an empty description is allowed and simply
  has no trailers. Locally the composed commit is judged only when the variables are
  set, so a developer can reproduce the CI result. Editing the description does not
  rerun the main CI workflow, so a separate small workflow reruns only this check when
  the description is edited.
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

**Amendment (2026-11-19): status `partial`.** A specification that is built only in
part, such as a library feature whose CLI commands exist while its GUI waits, has status
`partial` and names the criteria not built yet in the header field `- Pending: 1, 2, 8`.
Splitting it instead would move criteria to new IDs, and criterion numbers are never
reassigned (`specs/README.md`). The checks treat `partial` like `implemented` except
for the pending criteria: every other criterion needs a test, a test on a pending
criterion is an error (the field is then stale), open questions are not allowed, and
rule B applies. Only a `partial` specification may have `- Pending:`, and it must have
it.

**Amendment (2026-11-20): test levels and system tests for the command line.** A test's
level follows from where its marker sits: `system` under `apps/<app>/tests/` (the built
program), `integration` under a crate's `tests/` directory, `unit` anywhere else.
`cargo xtask spec-report` shows the levels of each criterion's tests in a column. Every
criterion of an `implemented` or `partial` `CLI-*` specification, except the pending ones,
needs a test under `apps/cli/tests/`, because the user observes it through `framehive`
itself. A criterion that has only lower-level tests today is listed in the header field
`- E2e-owed: 2, 4–6`; the list is debt, and a new entry needs a reason in its pull
request, such as hardware the binary cannot reach. A test under `apps/cli/tests/`
on a listed criterion is an error (the field is then stale), and only a `CLI-*`
specification may have the field. Listing the debt in each specification keeps it next to
the criteria it concerns; a central list would drift from them.

Example specification path: `specs/import/IMPORT-002-sequence-numbers.md`.

## Alternatives

- *Executable Cucumber/Gherkin*: an extra dependency and a second test language; the
  ID-based coupling achieves the same with what we already have.
- *`docs/dev/requirements/` only*: too coarse for unambiguous behavior and not coupled to
  tests.

## Consequences

- Every behavior is readable by humans in prose and backed by tests.
- More effort per change; in return, the specification never goes stale.
- The diff check is a heuristic: it forces specification and tests to be considered
  together, but it cannot check whether the prose is actually correct. That remains
  the agent's responsibility and is required by the workflow (`CLAUDE.md`).
