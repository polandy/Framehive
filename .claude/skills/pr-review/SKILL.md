---
name: pr-review
description: Risk-tiered review of one Framehive pull request against CLAUDE.md, its specs and the architecture — tests against the diff, CI (fix failures), branch freshness — posted as one PR comment. Use when asked to review a PR by number or branch.
argument-hint: <PR number or branch>
---

# Pull request review

Review the pull request in `$ARGUMENTS` (a number or a branch; without one, the pull request
of the current branch). The procedure around it is `docs/pull-requests.md`.

## Pick the tier first

Decide from `gh pr diff <PR> --name-only`, and name the tier in the verdict.

- **Light:** only `*.md`, `docs/`, `.claude/`, `.github/` or `assets/`. Check that the text is
  correct, English, consistent with the files it describes, and that no doc contradicts
  another one. Skip the test table.
- **Full:** anything else. Every step below.

## Rules for both tiers

- Read the diff before any file, and open a doc *section* (`grep -n` its heading, then read
  that range), never a whole directory.
- Trust a green CI: it ran `cargo xtask ci` on the head commit. Run it locally only after a
  fix of yours.
- Red CI: `cargo xtask ci-failures <PR>` first. Fall back to `gh run view --log-failed` only
  when that is not enough.
- Fix real defects on the pull request's branch, test first: a missing test, a wrong spec
  marker, a broken rule from `CLAUDE.md`. Report matters of taste, do not change them.
- Work in a checkout of the branch that holds no one else's uncommitted work; otherwise
  use your own worktree (`git worktree add ../framehive-review-<PR> <branch>`).

## 1. Context

- `gh pr view <PR> --json title,body,headRefName,mergeStateStatus,statusCheckRollup`.
- `gh pr diff <PR>`. Review the code against the intent the description states.
- A UI change needs a `## Screenshots` comment with images from `framehive-assets`
  (`docs/pull-requests.md` section 5). Missing screenshots block the review.
- The title must be a Conventional Commit subject, and the description must be pure commit
  body text with the right trailer, because both become the squash commit.

## 2. The standard

Follow `.claude/commands/review.md`, steps 2 and 3. `CLAUDE.md` and the architecture
sections it points to are the standard; a restated list here would go stale.

## 3. Tests against the diff (Full)

Build a table with one row per changed source file: the test that drives *its changed
lines* (file and test name) and its `/// spec: <ID>.<n>` marker. A row you cannot fill is a
finding, whatever the title says. A pull request often carries a second change, and that one
ships without tests.

## 4. CI

`gh pr checks <PR>`: both `xtask ci` jobs green. When red, fix the cause on the branch,
run `cargo xtask ci` locally, commit, push and wait for the new run. Never re-run a red job
unchanged (ADR 0006).

## 5. Freshness

`git fetch origin && git rev-list --count <head>..origin/main`. When behind, merge
`origin/main` into the branch and push; never rebase. Then repeat steps 2 to 4, because
`main` may have brought a spec or a decision the pull request now contradicts.

## 6. Verdict

1. **Summary:** what the pull request does, in one paragraph, and the tier.
2. **Test table** (Full).
3. **Findings:** per check ✅ ok, ⚠️ issue (`file:line`) or 🔧 fixed by me (commit).
4. **Blockers:** what must change before the merge that you could not fix.
5. **Merge readiness:** ready or not ready. Never merge: the owner decides, every time.

Post it in English, after your last push, as
`gh pr comment <PR> --body '## 🤖 PR quality review …'`. Replace an earlier verdict with
`gh pr comment <PR> --edit-last --body …` instead of adding a second one. Then remove your
review worktree, if you made one.
