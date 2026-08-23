#!/usr/bin/env python3
"""Stop: end the session if the acceptance test has gone green.

Green is a build failure in M0 — Foundations. The acceptance test is written
before the code it tests and must fail for the whole milestone, and an agent's
trained instinct is to make a red test pass. This hook is the mechanical answer
to that instinct: it does not ask the model to remember, it stops the session.

**Fails closed.** If the acceptance test exists and this hook cannot establish
that it failed, it blocks. A hook that shrugs when it cannot tell is worse than
no hook, because it reads as a clean bill of health.
"""
import json
import os
import subprocess
import sys

ACCEPTANCE_DIR = "tests/acceptance"
ACCEPTANCE_PACKAGE = "acceptance"


def block(reason: str) -> None:
    print(json.dumps({"decision": "block", "reason": reason}))
    sys.exit(0)


def main() -> None:
    try:
        payload = json.load(sys.stdin)
    except Exception:
        sys.exit(0)

    # A Stop hook that already blocked once must not block again, or the
    # session cannot end at all.
    if payload.get("stop_hook_active"):
        sys.exit(0)

    root = os.environ.get("CLAUDE_PROJECT_DIR")
    if not root:
        sys.exit(0)

    tests = os.path.join(root, ACCEPTANCE_DIR)
    present = os.path.isdir(tests) and any(
        name.endswith(".rs") for name in os.listdir(tests)
    )
    if not present:
        sys.exit(0)  # nothing to police yet — M0 step 9 writes it

    if not has_package(root, ACCEPTANCE_PACKAGE):
        block(
            f"{ACCEPTANCE_DIR}/ holds an acceptance test and there is no "
            f"`{ACCEPTANCE_PACKAGE}` package to run it with, so this hook cannot "
            "establish that it failed. M0 step 9 owns wiring how the test is "
            "invoked — here and in `xtask/src/rules.rs`, which has the same gap. "
            "Blocking rather than assuming."
        )

    result = subprocess.run(
        ["cargo", "test", "--package", ACCEPTANCE_PACKAGE, "--quiet"],
        cwd=root, capture_output=True, text=True,
    )
    if result.returncode == 0:
        block(
            "The acceptance test passed. Green is a build failure in M0 — "
            "Foundations: the test is written before the code it tests and must "
            "fail for the whole milestone. Something was stubbed, weakened or "
            "made to pass. Undo it and report what happened rather than fixing "
            "the test."
        )
    sys.exit(0)


def has_package(root: str, name: str) -> bool:
    """Whether the workspace has a package by that name, asked of cargo itself
    rather than inferred from the directory tree."""
    result = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        cwd=root, capture_output=True, text=True,
    )
    if result.returncode != 0:
        return False
    try:
        meta = json.loads(result.stdout)
    except Exception:
        return False
    return any(p.get("name") == name for p in meta.get("packages", []))


main()
