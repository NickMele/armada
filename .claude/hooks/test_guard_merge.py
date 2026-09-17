#!/usr/bin/env python3
"""What `guard_merge.py` refuses, and what it must let through.

`python3 .claude/hooks/test_guard_merge.py`. Nothing here runs git or `gh`: the
hook reads a payload and answers, so a test is one string in and one decision
out. The false-positive half is the half that matters — a hook that refuses an
ordinary branch push stops every agent in the repository.
"""
import json
import pathlib
import subprocess
import sys
import unittest

HOOK = pathlib.Path(__file__).with_name("guard_merge.py")


def decide(command: str) -> str | None:
    """The hook's decision on one Bash command, or None where it stayed silent."""
    run = subprocess.run(
        [sys.executable, str(HOOK)],
        input=json.dumps({"tool_name": "Bash", "tool_input": {"command": command}}),
        capture_output=True,
        text=True,
        check=True,
    )
    if not run.stdout.strip():
        return None
    answer = json.loads(run.stdout)["hookSpecificOutput"]
    return answer["permissionDecision"]


class Refuses(unittest.TestCase):
    def test_a_push_naming_the_base(self) -> None:
        for command in (
            "git push origin main",
            "git push origin HEAD:main",
            "git push origin HEAD:refs/heads/main",
            "git -C /Users/x/armada push origin main",
            "git push --force origin my-branch:main",
            "git push origin --delete main",
            "cd /tmp/x && git push origin main",
        ):
            with self.subTest(command=command):
                self.assertEqual(decide(command), "deny")

    def test_a_merge_pressed_by_hand(self) -> None:
        for command in (
            "gh pr merge 1327 --merge",
            "gh pr merge --squash 1327",
            "git fetch && gh pr merge 1327 --merge --delete-branch",
        ):
            with self.subTest(command=command):
                self.assertEqual(decide(command), "deny")

    def test_the_refusal_says_what_to_run_instead(self) -> None:
        run = subprocess.run(
            [sys.executable, str(HOOK)],
            input=json.dumps({"tool_input": {"command": "gh pr merge 1"}}),
            capture_output=True, text=True, check=True,
        )
        reason = json.loads(run.stdout)["hookSpecificOutput"]["permissionDecisionReason"]
        self.assertIn("scripts/land preflight", reason)
        self.assertIn("docs/capabilities/merge-line.md", reason)


class Allows(unittest.TestCase):
    def test_an_ordinary_branch_push(self) -> None:
        for command in (
            "git push -u origin tools/merge-line",
            "git push --force-with-lease origin fix/main-bar",
            "git push",
            "git push origin main:refs/heads/spike",
            "git -C /Users/x/armada push origin HEAD:refs/heads/tools/merge-line",
        ):
            with self.subTest(command=command):
                self.assertIsNone(decide(command))

    def test_the_line_itself_and_reading_a_pull_request(self) -> None:
        for command in (
            "scripts/land",
            "scripts/land --status",
            "gh pr view 1327 --json state",
            "gh pr list --state merged",
            "gh pr create --fill",
        ):
            with self.subTest(command=command):
                self.assertIsNone(decide(command))

    def test_prose_naming_the_commands(self) -> None:
        # A doc edit or a grep that carries the words is not a merge.
        for command in (
            "grep -rn 'gh pr merge' docs",
            "echo 'run git push origin main'",
        ):
            with self.subTest(command=command):
                self.assertIsNone(decide(command))

    def test_a_command_the_shell_could_not_parse(self) -> None:
        self.assertIsNone(decide("git push 'origin main"))


if __name__ == "__main__":
    unittest.main(verbosity=2)
