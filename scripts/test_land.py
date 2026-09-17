#!/usr/bin/env python3
#
# `scripts/land` against a throwaway repository with a local bare remote, a
# stub `gh` that merges into it with `--merge` as GitHub would, and a stub
# `armada` whose Checks are shell files in the tree being gated. Nothing here
# reaches GitHub or runs a real Check.
#
#   python3 scripts/test_land.py
#
# Beside the script because no Python test in this repository has a home yet.

import json
import os
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import unittest
from hashlib import sha256

LAND = os.path.join(os.path.dirname(os.path.abspath(__file__)), "land")

# `gh pr view` and `gh pr merge`, over a JSON file of pull requests. The merge
# refuses anything but `--merge --match-head-commit`, so a regression to a
# rebase or a squash fails here rather than on GitHub.
STUB_GH = r'''#!/usr/bin/env python3
import json, os, subprocess, sys, tempfile
state_file = os.environ["STUB_GH_STATE"]
remote = os.environ["STUB_REMOTE"]
prs = json.load(open(state_file))
args = sys.argv[1:]

def git(*a, cwd=None):
    return subprocess.run(["git", *a], cwd=cwd, capture_output=True, text=True, check=True).stdout.strip()

def head(branch):
    said = git("ls-remote", remote, f"refs/heads/{branch}")
    return said.split()[0] if said else None

def find(sel):
    for number, pr in prs.items():
        if sel == number or sel == pr["branch"]:
            return number, pr
    sys.exit("no pull requests found")

if args[:2] == ["pr", "view"]:
    number, pr = find(args[2])
    fields = args[args.index("--json") + 1].split(",")
    full = {
        "number": int(number),
        "state": pr["state"],
        "baseRefName": "main",
        "headRefOid": pr.get("merged_head") or head(pr["branch"]),
        "mergeCommit": {"oid": pr["merge_commit"]} if pr.get("merge_commit") else None,
    }
    print(json.dumps({f: full[f] for f in fields}))
elif args[:2] == ["pr", "merge"]:
    number, pr = find(args[2])
    for banned in ("--rebase", "--squash", "--delete-branch", "--admin"):
        if banned in args:
            sys.exit(f"stub gh: {banned} is not how this repository merges")
    if "--merge" not in args or "--match-head-commit" not in args:
        sys.exit("stub gh: --merge --match-head-commit is required")
    pinned = args[args.index("--match-head-commit") + 1]
    current = head(pr["branch"])
    if current != pinned:
        sys.exit(f"stub gh: head is {current}, not {pinned}")
    work = tempfile.mkdtemp()
    git("clone", "--quiet", remote, work)
    git("config", "user.name", "stub", cwd=work)
    git("config", "user.email", "stub@example.com", cwd=work)
    race = os.environ.get("STUB_GH_RACE")
    if race and os.path.exists(race):
        os.remove(race)
        with open(os.path.join(work, "race.txt"), "w") as out:
            out.write("pressed by hand\n")
        git("add", "race.txt", cwd=work)
        git("commit", "--quiet", "-m", "a merge nobody queued", cwd=work)
    git("fetch", "--quiet", "origin", pr["branch"], cwd=work)
    done = subprocess.run(["git", "merge", "--no-ff", "--no-edit", "FETCH_HEAD"], cwd=work, capture_output=True, text=True)
    if done.returncode != 0:
        sys.exit("stub gh: Pull request is not mergeable")
    git("push", "--quiet", "origin", "HEAD:main", cwd=work)
    pr.update(state="MERGED", merge_commit=git("rev-parse", "HEAD", cwd=work), merged_head=pinned)
    json.dump(prs, open(state_file, "w"))
    print(f"Merged pull request #{number}")
else:
    sys.exit(f"stub gh: {args} is not stubbed")
'''

# `covers` reads `checks.json` in the working directory: a Check name to a list
# of path prefixes, or null for always. `check <name>` runs `checks/<name>.sh`.
STUB_ARMADA = r'''#!/usr/bin/env python3
import json, os, subprocess, sys
args = sys.argv[1:]
if args == ["covers"]:
    paths = [p for p in sys.stdin.read().splitlines() if p]
    for name, prefixes in json.load(open("checks.json")).items():
        if prefixes is None or any(p.startswith(x) for p in paths for x in prefixes):
            print(name)
elif args[:1] == ["check"]:
    script = os.path.join("checks", f"{args[1]}.sh")
    sys.exit(subprocess.run(["sh", script]).returncode if os.path.exists(script) else 0)
elif args[:1] == ["run"]:
    sys.exit(0)
else:
    sys.exit(f"stub armada: {args}")
'''


def key(branch):
    return sha256(branch.encode()).hexdigest()[:16]


def load(path):
    with open(path) as held:
        return json.load(held)


def dump(path, value):
    with open(path, "w") as out:
        json.dump(value, out)


def sh(*argv, cwd=None, env=None, check=True):
    done = subprocess.run(argv, cwd=cwd, env=env, capture_output=True, text=True)
    if check and done.returncode != 0:
        raise AssertionError(f"{argv} exited {done.returncode}:\n{done.stdout}{done.stderr}")
    return done


class Line(unittest.TestCase):
    def setUp(self):
        self.root = os.path.realpath(tempfile.mkdtemp(prefix="land-"))
        self.remote = os.path.join(self.root, "remote.git")
        self.repo = os.path.join(self.root, "repo")
        self.prs = os.path.join(self.root, "prs.json")
        stubs = os.path.join(self.root, "bin")
        os.makedirs(stubs)
        for name, body in (("gh", STUB_GH), ("armada", STUB_ARMADA)):
            path = os.path.join(stubs, name)
            with open(path, "w") as out:
                out.write(body)
            os.chmod(path, 0o755)
        with open(self.prs, "w") as out:
            out.write("{}")
        self.env = dict(
            os.environ,
            ARMADA_LAND_GH=os.path.join(stubs, "gh"),
            ARMADA_LAND_ARMADA=os.path.join(stubs, "armada"),
            ARMADA_LAND_FOUNDATIONS="sh foundations.sh",
            ARMADA_LAND_SETUP="",
            ARMADA_LAND_SEED="",
            ARMADA_LAND_HEAD_WAIT="10",
            STUB_GH_STATE=self.prs,
            STUB_REMOTE=self.remote,
            GIT_CONFIG_GLOBAL="/dev/null",
            GIT_AUTHOR_NAME="test", GIT_AUTHOR_EMAIL="test@example.com",
            GIT_COMMITTER_NAME="test", GIT_COMMITTER_EMAIL="test@example.com",
        )
        sh("git", "init", "--quiet", "--bare", "-b", "main", self.remote)
        sh("git", "init", "--quiet", "-b", "main", self.repo)
        self.write(self.repo, {
            "checks.json": json.dumps({"test": None, "ui": ["ui/"]}),
            # Red only in combination: each branch alone passes.
            "checks/test.sh": "! { [ -f one.txt ] && [ -f two.txt ]; }\n",
            "foundations.sh": "cat foundations.txt 2>/dev/null; true\n",
            "foundations.txt": "a rule main already fails\n",
            "shared.txt": "base\n",
        })
        self.git(self.repo, "add", "-A")
        self.git(self.repo, "commit", "--quiet", "-m", "base")
        self.git(self.repo, "remote", "add", "origin", self.remote)
        self.git(self.repo, "push", "--quiet", "-u", "origin", "main")

    def tearDown(self):
        for line in (self.state_file("runner.log"),):
            if os.path.exists(line) and os.environ.get("LAND_TEST_VERBOSE"):
                print(open(line).read())
        # Only this test's runner, named by its own git directory.
        subprocess.run(["pkill", "-f", f"--runner {self.repo}/.git"], capture_output=True)
        shutil.rmtree(self.root, ignore_errors=True)

    # ------------------------------------------------------------ helpers

    def git(self, cwd, *args):
        return sh("git", *args, cwd=cwd, env=self.env).stdout.strip()

    def write(self, cwd, files):
        for path, body in files.items():
            os.makedirs(os.path.dirname(os.path.join(cwd, path)) or cwd, exist_ok=True)
            with open(os.path.join(cwd, path), "w") as out:
                out.write(body)

    def state_file(self, *parts):
        return os.path.join(self.repo, ".git", "armada-land", *parts)

    def branch(self, name, files):
        """An agent's worktree on `name`, cut from main, committed, pushed, with a PR."""
        where = os.path.join(self.root, "wt-" + name.replace("/", "-"))
        self.git(self.repo, "fetch", "--quiet", "origin")
        self.git(self.repo, "worktree", "add", "--quiet", "-b", name, where, "origin/main")
        self.commit(where, files, f"work on {name}")
        prs = load(self.prs)
        prs[str(len(prs) + 1)] = {"branch": name, "state": "OPEN"}
        dump(self.prs, prs)
        return where

    def commit(self, where, files, message):
        self.write(where, files)
        self.git(where, "add", "-A")
        self.git(where, "commit", "--quiet", "-m", message)
        self.git(where, "push", "--quiet", "-u", "origin", "HEAD")

    def land(self, where, *args, check=True):
        done = sh(sys.executable, LAND, *args, cwd=where, env=self.env, check=False)
        if check and done.returncode not in (0,):
            raise AssertionError(f"land {args} exited {done.returncode}:\n{done.stdout}{done.stderr}")
        return done

    def settle(self, where, branch, timeout=60):
        deadline = time.monotonic() + timeout
        while time.monotonic() < deadline:
            done = self.land(where, "--status", branch, check=False)
            if done.returncode != 3:
                return done
            time.sleep(0.2)
        raise AssertionError(f"{branch} still in line:\n{done.stdout}")

    def main_files(self):
        work = os.path.join(self.root, "check-" + str(time.monotonic_ns()))
        sh("git", "clone", "--quiet", self.remote, work, env=self.env)
        return set(os.listdir(work))

    def outcome(self, branch):
        return load(self.state_file("outcomes", key(branch) + ".json"))

    def logged(self, branch):
        """What the turn wrote a log for, apart from the merge itself."""
        return sorted(set(os.listdir(self.state_file("logs", key(branch)))) - {"merge.log"})

    # ------------------------------------------------------------ the claims

    def test_two_back_to_back_the_second_reruns_red_and_does_not_merge(self):
        one = self.branch("fix/one", {"one.txt": "1\n"})
        two = self.branch("fix/two", {"two.txt": "2\n"})
        self.land(one, "preflight")
        self.land(two, "preflight")
        self.land(one)
        self.land(two)

        first = self.settle(one, "fix/one")
        self.assertEqual(first.returncode, 0, first.stdout)
        self.assertEqual(self.logged("fix/one"), [], "main had not moved, so nothing reran")
        second = self.settle(two, "fix/two")
        self.assertEqual(second.returncode, 4, second.stdout)
        self.assertIn("test failed", second.stdout)
        self.assertIn("test.log", self.logged("fix/two"))

        on_main = self.main_files()
        self.assertIn("one.txt", on_main)
        self.assertNotIn("two.txt", on_main)
        self.assertEqual(self.git(two, "rev-parse", "HEAD"), self.git(two, "ls-remote", "origin", "refs/heads/fix/two").split()[0],
                         "a red gate pushes nothing onto the branch")
        self.assertEqual(load(self.prs)["2"]["state"], "OPEN")
        self.assertEqual(os.listdir(self.state_file("queue")), [], "every exit leaves the line")

    def test_main_unmoved_reruns_nothing(self):
        where = self.branch("fix/alone", {"checks/test.sh": "exit 1\n"})
        self.land(where, "preflight")
        self.land(where)
        done = self.settle(where, "fix/alone")
        self.assertEqual(done.returncode, 0, done.stdout)
        self.assertIn("merged as", done.stdout)
        self.assertEqual(self.logged("fix/alone"), [], "a failing Check never ran, because main had not moved")
        merge = self.outcome("fix/alone")["merge_commit"]
        self.git(self.repo, "fetch", "--quiet", "origin")
        self.assertEqual(len(self.git(self.repo, "rev-list", "--parents", "-n", "1", merge).split()), 3, "a merge commit, not a rebase")
        self.assertEqual(self.git(self.repo, "ls-remote", "origin", "refs/heads/fix/alone"), "", "the remote branch is deleted")
        self.assertIn("git worktree remove", done.stdout)
        self.assertTrue(os.path.isdir(where), "the agent's worktree is never removed")

    def test_a_conflict_stops_and_keeps_its_place(self):
        where = self.branch("fix/clash", {"shared.txt": "branch\n"})
        mover = self.branch("fix/mover", {"shared.txt": "moved\n"})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/mover").returncode, 0)

        self.land(where, "preflight")
        self.land(where)
        done = self.settle(where, "fix/clash")
        self.assertEqual(done.returncode, 5, done.stdout)
        self.assertIn("shared.txt", done.stdout)
        place = self.outcome("fix/clash")["place"]

        self.git(where, "fetch", "--quiet", "origin")
        sh("git", "merge", "origin/main", cwd=where, env=self.env, check=False)
        self.commit(where, {"shared.txt": "resolved\n"}, "resolve")
        self.land(where, "preflight")
        self.land(where)
        requeued = self.outcome("fix/clash")["place"]
        self.assertEqual(requeued, place, "resubmitted after a conflict, it keeps its place")
        self.assertEqual(self.settle(where, "fix/clash").returncode, 0)

    def test_a_conflicted_generated_file_is_regenerated_not_refused(self):
        header = "GENERATED by `sh regen.sh`. Do not hand-edit.\n"
        self.write(self.repo, {"regen.sh": "cat parts/* > gen.txt\n", "gen.txt": header, "parts/0": header})
        self.git(self.repo, "add", "-A")
        self.git(self.repo, "commit", "--quiet", "-m", "generated")
        self.git(self.repo, "push", "--quiet", "origin", "main")
        where = self.branch("fix/gen-a", {"parts/a": "a\n", "gen.txt": header + "a\n"})
        mover = self.branch("fix/gen-b", {"parts/b": "b\n", "gen.txt": header + "b\n"})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/gen-b").returncode, 0)
        self.land(where, "preflight")
        self.land(where)
        done = self.settle(where, "fix/gen-a")
        self.assertEqual(done.returncode, 0, done.stdout)
        work = os.path.join(self.root, "gen-check")
        sh("git", "clone", "--quiet", self.remote, work, env=self.env)
        self.assertEqual(open(os.path.join(work, "gen.txt")).read(), header + "a\nb\n")

    def test_a_killed_runner_gives_the_turn_up(self):
        marker = os.path.join(self.root, "slow-once")
        mover = self.branch("fix/first", {"first.txt": "1\n"})
        where = self.branch("fix/slow", {"checks/test.sh": f"[ -f {marker} ] && exit 0\ntouch {marker}\nsleep 60\n"})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/first").returncode, 0)
        self.land(where, "preflight")
        self.land(where)

        deadline = time.monotonic() + 30
        while not os.path.exists(marker):
            self.assertLess(time.monotonic(), deadline, "the slow Check never started")
            time.sleep(0.1)
        os.killpg(os.getpgid(self.outcome("fix/slow")["runner"]), signal.SIGKILL)

        done = self.settle(where, "fix/slow")  # --status starts a runner for the turn left behind
        self.assertEqual(done.returncode, 0, done.stdout)
        self.assertIn("first.txt", self.main_files())

    def test_status_while_waiting_answers_at_once(self):
        gate = os.path.join(self.root, "gate-open")
        mover = self.branch("fix/ahead", {"ahead.txt": "1\n"})
        slow = self.branch("fix/held", {"checks/test.sh": f"while [ ! -f {gate} ]; do sleep 0.1; done\n"})
        behind = self.branch("feature/behind/deep", {"behind.txt": "1\n"})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/ahead").returncode, 0)
        self.land(slow, "preflight")
        self.land(slow)
        self.land(behind, "preflight")
        self.land(behind)

        started = time.monotonic()
        done = self.land(behind, "--status", check=False)
        self.assertLess(time.monotonic() - started, 1.0)
        self.assertEqual(done.returncode, 3, done.stdout)
        self.assertIn("feature/behind/deep: waiting", done.stdout)
        self.assertIn("1. fix/held", done.stdout)
        self.assertIn("2. feature/behind/deep", done.stdout)

        open(gate, "w").close()
        self.assertEqual(self.settle(slow, "fix/held").returncode, 0)
        self.assertEqual(self.settle(behind, "feature/behind/deep").returncode, 0)

    def test_a_merge_pressed_in_between_is_reported_ungated(self):
        race = os.path.join(self.root, "race")
        open(race, "w").close()
        self.env["STUB_GH_RACE"] = race
        where = self.branch("fix/raced", {"raced.txt": "1\n"})
        self.land(where, "preflight")
        self.land(where)
        done = self.settle(where, "fix/raced")
        self.assertEqual(done.returncode, 6, done.stdout)
        self.assertIn("ungated combination", done.stdout)

    def test_only_a_new_foundations_line_is_red(self):
        mover = self.branch("fix/move", {"moved.txt": "1\n"})
        worse = self.branch("fix/worse", {"foundations.txt": "a rule main already fails\na new rule broken\n"})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/move").returncode, 0)
        self.land(worse, "preflight")
        self.land(worse)
        done = self.settle(worse, "fix/worse")
        self.assertEqual(done.returncode, 4, done.stdout)
        self.assertIn("a new rule broken", done.stdout)
        self.assertNotIn("  a rule main already fails", done.stdout)

    def test_a_dirty_tree_or_a_missing_stamp_is_refused(self):
        where = self.branch("fix/dirty", {"x.txt": "1\n"})
        self.assertEqual(self.land(where, check=False).returncode, 1)
        self.write(where, {"stray.txt": "untracked\n"})
        done = self.land(where, "preflight", check=False)
        self.assertEqual(done.returncode, 1)
        self.assertIn("stray.txt", done.stderr)


if __name__ == "__main__":
    unittest.main()
