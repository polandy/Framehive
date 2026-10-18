Run a quick project health check and report it as a short dashboard:

1. `nix develop -c cargo xtask ci` — the same gate as CI; it prints one line per step and the
   failing step in full. Delegate it to a subagent (`model: "sonnet"`) that reports only the
   result lines.
2. `git log --oneline -5` and `git status --short` in one call.
3. The open items in `TODO.md` (count, and the oldest three).
4. Spec status counts: `grep -h '^- Status:' specs/*/*.md | sort | uniq -c`.

Flag anything that needs attention: a failing step, a warning from our checks, uncommitted
work, a spec marked `implemented` while its tests are missing.
