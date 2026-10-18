# 0019 – Review screenshots are linked from a comment

- Status: accepted
- Date: 2026-10-18
- Supersedes: decision 9 of ADR 0017, in where the screenshots are linked

## Context

ADR 0017 keeps review screenshots in the separate repository `framehive-assets`, under
`pr-<n>/`, and links them from the pull request description. Pull requests are
squash-merged, and the description becomes the commit body word for word
(`docs/pull-requests.md`). The links would then stay in the history of `main` for good,
while ADR 0017 itself calls the images disposable and allows pruning the repository at
any time. Every pruning would leave broken links in commit messages.

## Decision

1. **Review screenshots are linked from a comment on the pull request**, never from its
   description. The comment is headed `## Screenshots` and holds 2–3 small, cropped images
   with one line each on what they show.
2. Everything else in decision 9 of ADR 0017 stays: the images live in
   `polandy/framehive-assets` under `pr-<n>/`, nothing pins them, and the repository may be
   pruned or recreated at any time.
3. A pull request that changes the UI is not ready for review without that comment.

## Alternatives

- **Links in the description, as in ADR 0017:** one place to look, but disposable URLs
  end up in the permanent history and break there.
- **Images uploaded to the comment through the GitHub web interface:** no second
  repository, but agents cannot upload that way with `gh`, and the files land on a
  GitHub host outside our control.

## Consequences

- The commit history of `main` holds no links to disposable images.
- Agents need push access to `framehive-assets`; the repository has no branch
  protection, because its content carries no weight.
