Identify the next task and plan it:

1. Sources of open work, in this order:
   - whatever the owner has just asked for
   - `TODO.md` (open items only, oldest first unless one blocks another)
   - specs with status `accepted`, which are ready to implement: `grep -l 'Status: accepted' specs/*/*.md`
   - the current roadmap phase in `docs/requirements/10-roadmap.md`
2. Open questions are not tasks for you; they need the owner. List them only if one blocks the
   chosen task: `grep -n -A4 '^## Open questions' specs/*/*.md`.
3. Read ONLY the spec, the requirement sections and the ADRs that the item references, never a
   whole directory.
4. Propose a plan of at most 5 steps. Each step is one Conventional Commit with a green
   `cargo xtask ci`. Spec first (CLAUDE.md, Workflow), and the last step updates the spec
   status, the crate README, `CHANGELOG.md` for user-visible changes, and removes the item from
   `TODO.md`.
5. Name the decisions the owner has to make, each with your recommendation. Do not start
   implementing until the owner agrees.
