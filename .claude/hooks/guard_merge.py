#!/usr/bin/env python3
"""PreToolUse: a merge to `main` goes through the line, never by hand.

Two green branches can each pass every Check and still leave `main` red, because
each ran its Checks against a `main` that had moved by the time it merged. That
is what `scripts/land` exists to stop, and it only holds while every merge goes
through it — so this refuses the two commands that reach `main` around it.

Reads the hook payload on stdin and answers `deny` or nothing at all.
`docs/capabilities/merge-line.md` is the design.
"""
import json
import shlex
import sys

# What `land` puts on the base. Both spellings of the same ref, plus the bare
# name a refspec may use.
BASE = "main"
BASE_REFS = (BASE, f"refs/heads/{BASE}")

SAY = (
    "Merges to `main` go through `scripts/land`, which reruns the Checks a "
    "moved `main` hits — the check a merge by hand skips, and the reason two "
    "green branches can leave `main` red.\n"
    "  scripts/land preflight   # once the branch's own Checks pass\n"
    "  scripts/land             # joins the line and returns\n"
    "  scripts/land --status    # poll this until it stops exiting 3\n"
    "Run it once the owner has said to merge. "
    "docs/capabilities/merge-line.md says what it does."
)


def answer(reason: str) -> None:
    """Emit a refusal and exit. Silence is this hook's every other answer."""
    print(json.dumps({
        "hookSpecificOutput": {
            "hookEventName": "PreToolUse",
            "permissionDecision": "deny",
            "permissionDecisionReason": reason,
        }
    }))
    sys.exit(0)


def segments(command: str) -> list[list[str]]:
    """The command split into the separate commands a shell would run.

    `cd x && git push origin main` is one string to the tool and two commands
    to the shell, and only the second one is this hook's business.
    """
    try:
        words = shlex.split(command, comments=True)
    except ValueError:
        # Unbalanced quotes: the shell would refuse it too.
        return []
    out: list[list[str]] = [[]]
    for word in words:
        if word in ("&&", "||", ";", "|", "&"):
            out.append([])
        else:
            out[-1].append(word)
    return [s for s in out if s]


def lands_on_base(words: list[str]) -> bool:
    """Whether this `git push` would write the base branch.

    A push names its destination in the refspec's right-hand side, and with no
    colon the whole word is both sides. `--delete main` is the same write by
    another route.
    """
    after_flags = [w for w in words[1:] if not w.startswith("-")]
    if not after_flags:
        # `git push` with no arguments follows the branch's upstream, which is
        # never `main` here: the checkout at `main` is never worked in.
        return False
    deleting = any(w in ("--delete", "-d") for w in words)
    # The first bare word is the remote; the rest are refspecs.
    for spec in after_flags[1:]:
        destination = spec.split(":")[-1] if ":" in spec else spec
        if destination in BASE_REFS:
            return True
        if deleting and spec in BASE_REFS:
            return True
    return False


def inner(words: list[str]) -> list[list[str]]:
    """The commands inside a `sh -c '…'`, which are commands like any other.

    A shell started by hand is the obvious way past a guard that reads the
    command it is given, and it costs one line to follow it in.
    """
    if not words or not words[0].rsplit("/", 1)[-1] in ("sh", "bash", "zsh"):
        return []
    try:
        script = words[words.index("-c") + 1]
    except (ValueError, IndexError):
        return []
    return segments(script)


def main() -> None:
    try:
        payload = json.load(sys.stdin)
    except Exception:
        sys.exit(0)

    command = (payload.get("tool_input") or {}).get("command") or ""
    if not command:
        sys.exit(0)

    commands = segments(command)
    for words in list(commands):
        commands.extend(inner(words))

    for words in commands:
        # `NAME=value git push …` runs git with an environment, and the
        # assignments sit where the program name would be.
        while words and "=" in words[0].split("/")[0] and not words[0].startswith("-"):
            words = words[1:]
        bare = [w for w in words if not w.startswith("-")]
        if len(bare) >= 2 and bare[0].endswith("git"):
            # `-C <path>` puts a path where a verb would be, so the verb is
            # whichever of the first few words git actually knows.
            if "push" in bare[1:4] and lands_on_base(words):
                answer(f"This pushes `{BASE}`.\n{SAY}")
        # `gh -R owner/repo pr merge` puts the repository between the two, so
        # the pair is looked for wherever it sits rather than at fixed places.
        if bare and bare[0].endswith("gh"):
            pairs = zip(bare, bare[1:])
            if any(verb == "pr" and act == "merge" for verb, act in pairs):
                answer(f"This merges a pull request by hand.\n{SAY}")
            # The same write as a request: `gh api -X PUT repos/…/pulls/1/merge`.
            if any(w.endswith("/merge") and "/pulls/" in w for w in bare):
                answer(f"This merges a pull request through the forge's API.\n{SAY}")

    sys.exit(0)


if __name__ == "__main__":
    main()
