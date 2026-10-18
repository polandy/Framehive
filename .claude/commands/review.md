Review uncommitted or recently committed changes against the project's standards:

1. `git diff HEAD` (or `git diff HEAD~1` if the working tree is clean). Read the diff before any
   file, and then only the files and doc sections the diff touches.
2. The standard is `CLAUDE.md` and the sections of `docs/architecture/` it points to. Check the
   diff against those files, not against a list restated here; a restated copy goes stale
   without anything turning red.
3. Beyond what `cargo xtask ci` enforces, check by reading:
   - Does every new behaviour have a test whose name says which rule would break, marked
     `/// spec: <ID>.<n>`?
   - Is the spec changed in the same change, or does the commit carry `Spec-Unchanged:` or
     `Spec-Only:` (ADR 0010)?
   - Are glossary terms used (`docs/architecture/10-glossary.md`), with no new synonyms?
   - Does an architecture change come with an ADR (CLAUDE.md rule 6)?
   - Do comments explain *why*, and does every public item have a doc comment?
   - Is a user-visible change in `CHANGELOG.md`, and is a closed `TODO.md` item removed?
4. Report concisely: what needs fixing, with `file:line`. If everything is clean, say so in one
   line.
