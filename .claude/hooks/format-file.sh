#!/bin/sh
# SPDX-FileCopyrightText: 2026 Framehive contributors
# SPDX-License-Identifier: GPL-3.0-or-later
#
# PostToolUse(Edit|Write): format a touched Rust file with rustfmt, so that the
# format step of `cargo xtask ci` does not fail later and cost another round trip.
#
# A missing tool is reported once per session (exit 2 shows stderr to the agent);
# after that the hook stays quiet, so a bare machine costs one line, not one per edit.
input=$(cat)

warn_once() {
  session=$(printf '%s' "$input" | sed -n 's/.*"session_id" *: *"\([^"]*\)".*/\1/p')
  marker="${TMPDIR:-/tmp}/framehive-format-hook-${session:-unknown}"
  [ -e "$marker" ] && exit 0
  : >"$marker"
  echo "$1" >&2
  exit 2
}

command -v jq >/dev/null 2>&1 || warn_once "format hook: jq not found, Rust files are not formatted on edit"
file=$(printf '%s' "$input" | jq -r '.tool_input.file_path // empty')
[ -n "$file" ] && [ -f "$file" ] || exit 0

case "$file" in
  "$CLAUDE_PROJECT_DIR"/*.rs) ;;
  *) exit 0 ;;
esac
case "$file" in
  "$CLAUDE_PROJECT_DIR"/target/*) exit 0 ;;
esac

command -v rustfmt >/dev/null 2>&1 || warn_once "format hook: rustfmt not found; run inside \`nix develop\`"

cd "$CLAUDE_PROJECT_DIR" || exit 0
rustfmt --edition 2024 "$file" >/dev/null 2>&1
exit 0
