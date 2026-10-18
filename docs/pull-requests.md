# Pull requests

Every change reaches `main` through a pull request. A ruleset enforces this, so the owner
cannot push to `main` directly either (ADR 0018). This file is the procedure for agents
and humans alike.

## 1. Branch

- One task, one branch, created from a fresh `origin/main`:
  `git fetch origin && git switch -c <type>/<slug> origin/main`.
  `<type>` is the Conventional Commit type of the change (`feat`, `fix`, `docs`, `ci`,
  …), and `<slug>` is a few words in kebab case.
- Agents working in parallel each use their own worktree next to the checkout
  (`git worktree add ../framehive-<slug> -b <type>/<slug> origin/main`), never a branch
  switch under someone else's uncommitted work.

## 2. Before opening

- `cargo xtask ci` is green locally. CI runs exactly the same command.
- The spec, the docs, `CHANGELOG.md` and `TODO.md` are part of the same change
  (`CLAUDE.md`, Workflow).
- Commit, push and open the pull request only when the owner asked for it. An approval
  for one of these steps does not cover the next.

## 3. Title and description

The pull request is squash-merged. Its title becomes the commit subject and its
description becomes the commit body, word for word. So:

- **Title:** a Conventional Commit subject in English, such as
  `feat(import): skip files already in the catalog`.
- **Description:** only commit body text. It explains *why*, and it ends with the
  `Spec-Unchanged:` or `Spec-Only:` trailer when ADR 0010 asks for one.
- Notes for the reviewer, open questions and test evidence go into a comment, not into
  the description.
- No `Co-Authored-By` or "Generated with" lines.

## 4. Review and CI

- CI runs `cargo xtask ci` on Linux and macOS. Both jobs must pass on the latest `main`.
- When CI is red, `cargo xtask ci-failures [<pr>]` prints only the failed step of each
  failed job. Fix the cause on the branch and push again. Never re-run a red job in the
  hope that it passes (ADR 0006).
- A pull request that changes the UI adds a comment with its screenshots (section 5)
  before it is reviewed.
- An agent reviews the pull request with the `pr-review` skill, or the
  `framehive-reviewer` agent, and posts the verdict as one comment.
- Review threads must be resolved before the merge.

## 5. Screenshots for UI changes

The images live in the separate, public repository `polandy/framehive-assets`, never
in this one (ADR 0017, ADR 0019). They are disposable, so the repository may be pruned.

1. Take 2–3 PNG screenshots of the changed screens, cropped to the change and at most
   1200 px wide. Show the light and the dark theme when the change affects colours.
2. Commit them to `framehive-assets` under `pr-<n>/`, in a checkout next to this one:

   ```sh
   git -C ../framehive-assets pull --ff-only
   mkdir -p ../framehive-assets/pr-<n> && cp <files> ../framehive-assets/pr-<n>/
   git -C ../framehive-assets add pr-<n>
   git -C ../framehive-assets commit -m "add screenshots for Framehive#<n>"
   git -C ../framehive-assets push
   ```

3. Comment on the pull request with a `## Screenshots` heading and one line per image:
   `![<what it shows>](https://raw.githubusercontent.com/polandy/framehive-assets/main/pr-<n>/<file>.png)`.
   Never link them from the description: it becomes the squash commit, and the links
   would break in the history once the images are pruned.

A new revision of the UI replaces the files and edits the same comment.

## 6. Keep it up to date

The ruleset requires the branch to contain the latest `main`. When it is behind, merge
`origin/main` into it and push. Do not rebase a pushed branch: a merge needs no force push
and keeps review comments anchored, and the squash merge removes the merge commit anyway.

## 7. Merge

- Only the owner decides when a pull request is merged, every time. An agent merges only
  on an explicit go-ahead for that pull request.
- Squash merge: `gh pr merge <pr> --squash`. The branch is deleted automatically.
- Afterwards, update the local `main` (`git switch main && git pull --ff-only`) and remove
  the worktree, if one was used.
