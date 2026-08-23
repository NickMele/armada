#!/usr/bin/env python3
"""PreToolUse: the three write rules, enforced before the write happens.

Reads the hook payload on stdin and answers with a permission decision:

  deny  — the write must not happen
  ask   — the write may happen once a human says so
  allow — say nothing, let it through

The rules are the gate's rules, moved earlier. `cargo xtask verify-foundations`
catches a violation after it is committed; this catches it before it is written,
which is the difference between a gate and a habit.

**This hook may not reach outside the plugin.** A copied plugin has no path back
to the repository it was installed from, so the repo is located through
`CLAUDE_PROJECT_DIR`, which the harness sets. With no project directory there is
nothing to police and the hook stays silent.
"""
import json
import os
import sys

WARN_LINES = 500
FAIL_LINES = 900

# A CLAUDE.md is read by an agent at the start of every task, so it earns a far
# tighter ceiling than source does. `xtask/src/rules.rs` carries the same two
# numbers and the reasoning; the two must not drift.
CLAUDE_MD_WARN = 30
CLAUDE_MD_FAIL = 50

# Untyped JSON is allowed exactly where bytes enter the process — plus the
# gate itself, which names the pattern it forbids and so always matches itself.
# `xtask/src/rules.rs` carries the same exemption; the two must not drift.
JSON_ALLOWED = ("crates/store/", "crates/ipc/", "xtask/")


def answer(decision: str, reason: str) -> None:
    """Emit a PreToolUse decision and exit. `allow` is silent by convention."""
    print(json.dumps({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": decision,
            "permissionDecisionReason": reason,
        }
    }))
    sys.exit(0)


def main() -> None:
    try:
        payload = json.load(sys.stdin)
    except Exception:
        sys.exit(0)

    root = os.environ.get("CLAUDE_PROJECT_DIR")
    if not root:
        sys.exit(0)

    tool_input = payload.get("tool_input") or {}
    path = tool_input.get("file_path") or tool_input.get("notebook_path") or ""
    if not path:
        sys.exit(0)

    try:
        rel = os.path.relpath(path, root).replace("\\", "/")
    except ValueError:
        sys.exit(0)
    if rel.startswith(".."):
        sys.exit(0)  # outside the repo: not this gate's business

    # ---- rule: every file under crates/*/src/ is in the manifest ----------
    parts = rel.split("/")
    under_crate_src = len(parts) > 3 and parts[0] == "crates" and parts[2] == "src"
    if under_crate_src:
        manifest = os.path.join(root, "foundations-manifest.txt")
        listed = set()
        try:
            with open(manifest) as f:
                listed = {line.strip() for line in f if line.strip()}
        except OSError:
            answer("deny", "foundations-manifest.txt is missing, so no write "
                           "under crates/*/src/ can be checked against it.")
        if rel not in listed:
            answer("deny", f"{rel} is not in foundations-manifest.txt. Add the "
                           "path to the manifest, sorted, in the same change "
                           "that adds the file — the manifest is how a file "
                           "nobody decided on becomes visible as a diff.")

    # ---- rule: no serde_json::from_* outside store and ipc ---------------
    written = tool_input.get("content") or tool_input.get("new_string") or ""
    if "serde_json::from_" in written and not rel.startswith(JSON_ALLOWED):
        answer("deny", f"{rel} would contain `serde_json::from_*`. Untyped JSON "
                       "belongs to `store` and `ipc`, the two places bytes enter "
                       "the process; everywhere else a value arrives typed.")

    # ---- rule: a CLAUDE.md routes, it does not explain -------------------
    lines = projected_lines(root, rel, tool_input)
    if rel.endswith("CLAUDE.md") and lines is not None:
        if lines > CLAUDE_MD_FAIL:
            answer("deny", f"{rel} would be about {lines} lines, over "
                           f"{CLAUDE_MD_FAIL}. A CLAUDE.md routes; it does not "
                           "explain. Move the explanation to a practice doc, a "
                           "skill or Notion and leave the pointer. v1's agent "
                           "file reached 328 lines one reasonable paragraph at "
                           "a time.")
        if lines > CLAUDE_MD_WARN:
            answer("ask", f"{rel} would be about {lines} lines, over "
                          f"{CLAUDE_MD_WARN}. Is this a pointer, or an "
                          "explanation that belongs somewhere it can be found "
                          "on purpose?")

    # ---- rule: 500 warns and needs acknowledgment, 900 fails -------------
    if lines is not None:
        if lines > FAIL_LINES:
            answer("deny", f"{rel} would be about {lines} lines, over {FAIL_LINES}. "
                           "Split it.")
        if lines > WARN_LINES:
            answer("ask", f"{rel} would be about {lines} lines, over {WARN_LINES}. "
                          "A long file is where the reasoning stopped fitting in "
                          "one place. Say so deliberately, or split it.")

    sys.exit(0)


def projected_lines(root: str, rel: str, tool_input: dict) -> "int | None":
    """Line count the file would have after this write.

    Exact for `Write`, which carries the whole file. **Estimated for `Edit`**,
    from the delta between the strings being swapped — the hook runs before the
    edit, so the real result does not exist yet. The estimate is only ever used
    against a threshold, and being a line or two out at 500 does not change the
    answer.
    """
    content = tool_input.get("content")
    if content is not None:
        return len(content.splitlines())

    old, new = tool_input.get("old_string"), tool_input.get("new_string")
    if old is None or new is None:
        return None
    try:
        with open(os.path.join(root, rel)) as f:
            current = len(f.read().splitlines())
    except OSError:
        return None
    delta = len(new.splitlines()) - len(old.splitlines())
    if tool_input.get("replace_all"):
        return None  # occurrence count is unknown before the edit runs
    return current + delta


main()
