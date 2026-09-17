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
import textwrap
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
        "baseRefName": os.environ.get("STUB_GH_BASE", "main"),
        "headRefOid": os.environ.get("STUB_GH_STALE_HEAD") or pr.get("merged_head") or head(pr["branch"]),
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
    if os.environ.get("STUB_GH_KILL_AFTER_MERGE"):
        os.kill(os.getppid(), 9)  # a runner killed between the merge and the proof
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
    open(f"{args[1]}.stamp", "w").write("prepared\n")
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
            ARMADA_LAND_SETUP="marker",
            ARMADA_LAND_SEED="seeded",
            LAND_TEST_EVIDENCE=os.path.join(self.root, "evidence.txt"),
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
            "checks/test.sh": (
                'printf "%s seed=%s setup=%s at=%s\\n" check '
                '"$([ -f seeded/mark.txt ] && echo yes || echo no)" '
                '"$([ -f marker.stamp ] && echo yes || echo no)" "$PWD" >> "$LAND_TEST_EVIDENCE"\n'
                "! { [ -f one.txt ] && [ -f two.txt ]; }\n"
            ),
            "foundations.sh": (
                'printf "%s seed=%s setup=%s at=%s\\n" foundations '
                '"$([ -f seeded/mark.txt ] && echo yes || echo no)" '
                '"$([ -f marker.stamp ] && echo yes || echo no)" "$PWD" >> "$LAND_TEST_EVIDENCE"\n'
                "cat foundations.txt 2>/dev/null; true\n"
            ),
            "foundations.txt": "FAIL  a rule main already fails\n        missing: its subject\n\nverify-foundations: RED — 1 failing, 0 warning\n",
            "shared.txt": "base\n",
            ".gitignore": ".armada/\n*.stamp\n",
        })
        self.git(self.repo, "add", "-A")
        self.git(self.repo, "commit", "--quiet", "-m", "base")
        self.git(self.repo, "remote", "add", "origin", self.remote)
        self.git(self.repo, "push", "--quiet", "-u", "origin", "main")
        # A seed path git neither tracks nor ignores, which is what the gate
        # worktree's own build directory is not.
        self.write(self.repo, {"seeded/mark.txt": "cloned into every gate worktree\n"})

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

    def main_clone(self):
        work = os.path.join(self.root, "check-" + str(time.monotonic_ns()))
        sh("git", "clone", "--quiet", self.remote, work, env=self.env)
        return work

    def main_files(self):
        return set(os.listdir(self.main_clone()))

    def main_head(self):
        return sh("git", "ls-remote", self.remote, "refs/heads/main", env=self.env).stdout.split()[0]

    def mover(self):
        """A script that pushes one commit onto main, for a base that moves mid-gate."""
        path = os.path.join(self.root, "move-main.sh")
        self.write(self.root, {"move-main.sh": textwrap.dedent("""\
            set -e
            [ -n "$LAND_TEST_MOVE_ONCE" ] && [ -f "$LAND_TEST_MOVE_ONCE" ] && exit 0
            [ -n "$LAND_TEST_MOVE_ONCE" ] && touch "$LAND_TEST_MOVE_ONCE"
            d=$(mktemp -d)
            git clone -q "$STUB_REMOTE" "$d"
            cd "$d"
            n=$(date +%s)$$
            echo "$n" > "moved-$n.txt"
            git add -A
            git commit -q -m "main moved under a gate"
            git push -q origin HEAD:main
        """)})
        self.env["LAND_TEST_MOVER"] = path
        return path

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
        self.assertNotIn("test.log", self.logged("fix/one"), "main had not moved, so no Check reran")
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

    def test_main_unmoved_runs_the_gate_and_no_check(self):
        where = self.branch("fix/alone", {"checks/test.sh": "exit 1\n"})
        self.land(where, "preflight")
        self.land(where)
        done = self.settle(where, "fix/alone")
        self.assertEqual(done.returncode, 0, done.stdout)
        self.assertIn("merged as", done.stdout)
        logged = self.logged("fix/alone")
        self.assertNotIn("test.log", logged, "a failing Check never ran, because main had not moved")
        self.assertIn("foundations.log", logged, "the gate reads the tree on every turn")
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
        landed = self.main_files()
        self.assertIn("raced.txt", landed, "it did land, which is the point")
        self.assertIn("race.txt", landed, "on top of the commit nobody gated it against")
        self.assertNotEqual(self.git(where, "ls-remote", "origin", "refs/heads/fix/raced"), "",
                            "the branch stays, because what landed was never checked")

    def test_only_a_new_failing_foundations_line_is_red(self):
        known = "FAIL  a rule main already fails\n        missing: its subject\n"
        mover = self.branch("fix/move", {"moved.txt": "1\n"})
        warned = self.branch("fix/warned", {"foundations.txt": known + "        warn:    a new warning\n\nverify-foundations: RED — 1 failing, 1 warning\n"})
        worse = self.branch("fix/worse", {"foundations.txt": "FAIL  a new rule\n        missing: a new subject\n" + known + "\nverify-foundations: RED — 1 failing, 0 warning\n"})
        for where, name in ((mover, "fix/move"), (warned, "fix/warned")):
            self.land(where, "preflight")
            self.land(where)
            done = self.settle(where, name)
            self.assertEqual(done.returncode, 0, done.stdout)
        self.land(worse, "preflight")
        self.land(worse)
        done = self.settle(worse, "fix/worse")
        self.assertEqual(done.returncode, 4, done.stdout)
        self.assertIn("missing: a new subject", done.stdout)
        self.assertNotIn("its subject", done.stdout.replace("a new subject", ""))

    def test_a_check_only_one_side_hits_still_reruns(self):
        """The union: main lands under `ui/`, the branch never touches it."""
        mover = self.branch("fix/ui-landed", {"ui/button.ts": "1\n"})
        where = self.branch("fix/no-ui", {"elsewhere.txt": "1\n"})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/ui-landed").returncode, 0)
        self.land(where, "preflight")
        self.land(where)
        self.assertEqual(self.settle(where, "fix/no-ui").returncode, 0)
        self.assertIn("ui.log", self.logged("fix/no-ui"), "a Check only the base hits still reruns")

    def test_what_a_regeneration_writes_besides_the_conflict_is_merged(self):
        header = "GENERATED by `sh regen.sh`. Do not hand-edit.\n"
        self.write(self.repo, {
            "regen.sh": "cat parts/* > gen.txt\nls parts > index.txt\n",
            "gen.txt": header, "index.txt": "0\n", "parts/0": header,
        })
        self.git(self.repo, "add", "-A")
        self.git(self.repo, "commit", "--quiet", "-m", "generated, with a second output")
        self.git(self.repo, "push", "--quiet", "origin", "main")
        where = self.branch("fix/gen-a", {"parts/a": "a\n", "gen.txt": header + "a\n"})
        mover = self.branch("fix/gen-b", {"parts/b": "b\n", "gen.txt": header + "b\n"})
        for wt, name in ((mover, "fix/gen-b"), (where, "fix/gen-a")):
            self.land(wt, "preflight")
            self.land(wt)
            self.assertEqual(self.settle(wt, name).returncode, 0)
        landed = self.main_clone()
        self.assertEqual(open(os.path.join(landed, "gen.txt")).read(), header + "a\nb\n")
        self.assertEqual(open(os.path.join(landed, "index.txt")).read(), "0\na\nb\n",
                         "the regeneration's other output is in the commit, not only in the gate")

    def test_a_foundations_run_that_names_no_rule_is_red(self):
        mover = self.branch("fix/moves-1", {"moved.txt": "1\n"})
        broken = self.branch("fix/breaks-the-gate", {"foundations.sh": "echo 'error[E0433]: cannot find `covers`' >&2\nexit 101\n"})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/moves-1").returncode, 0)
        self.land(broken, "preflight")
        self.land(broken)
        done = self.settle(broken, "fix/breaks-the-gate")
        self.assertEqual(done.returncode, 4, done.stdout)
        self.assertIn("naming no failing rule", done.stdout)
        self.assertIn("test.log", self.logged("fix/breaks-the-gate"),
                      "a gate that could not run does not hide the Checks beside it")
        self.assertNotIn("moved.txt", self.main_files() - {"moved.txt"} or set())

    def test_a_branch_that_breaks_the_gate_is_refused_on_an_unmoved_turn(self):
        where = self.branch("fix/breaks-the-gate-alone", {"foundations.sh": "exit 101\n"})
        self.land(where, "preflight")
        self.land(where)
        done = self.settle(where, "fix/breaks-the-gate-alone")
        self.assertEqual(done.returncode, 4, done.stdout)
        self.assertNotIn("foundations.sh", self.main_files() - {"foundations.sh"} or set())
        self.assertNotIn("test.log", self.logged("fix/breaks-the-gate-alone"), "and still no Check ran")

    def test_a_base_whose_own_gate_cannot_run_stops_rather_than_reds(self):
        # Broken on main by a hand merge, which is what the guard hook refuses.
        hand = os.path.join(self.root, "hand-merge")
        sh("git", "clone", "--quiet", self.remote, hand, env=self.env)
        self.write(hand, {"foundations.sh": "exit 101\n"})
        self.git(hand, "add", "-A")
        self.git(hand, "commit", "--quiet", "-m", "break the gate on main")
        self.git(hand, "push", "--quiet", "origin", "HEAD:main")
        after = self.branch("fix/after-base", {"after.txt": "1\n"})
        self.land(after, "preflight")
        self.land(after)
        done = self.settle(after, "fix/after-base")
        self.assertEqual(done.returncode, 7, done.stdout)
        self.assertIn("main itself is broken", done.stdout)
        self.assertIn("not at fault", done.stdout, "the branch behind a broken main is not the one to fix")

    def test_a_cached_base_run_that_is_not_a_report_is_taken_again(self):
        mover = self.branch("fix/moves-2", {"moved.txt": "1\n"})
        where = self.branch("fix/after-empty-cache", {"after.txt": "1\n"})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/moves-2").returncode, 0)
        os.makedirs(self.state_file("foundations"), exist_ok=True)
        with open(self.state_file("foundations", self.main_head() + ".txt"), "w") as out:
            out.write("error: the runner was killed half-way through\n")  # never a report
        self.land(where, "preflight")
        self.land(where)
        done = self.settle(where, "fix/after-empty-cache")
        self.assertEqual(done.returncode, 0, done.stdout)

    def test_a_renumbered_finding_is_not_a_new_one(self):
        known = "FAIL  a rule main already fails\n        missing: a/b.rs:10 — over 500\n"
        self.write(self.repo, {"foundations.txt": known})
        self.git(self.repo, "add", "-A")
        self.git(self.repo, "commit", "--quiet", "-m", "a numbered finding")
        self.git(self.repo, "push", "--quiet", "origin", "main")
        mover = self.branch("fix/moves-3", {"moved.txt": "1\n"})
        shifted = self.branch("fix/shifts-a-line", {"foundations.txt": known.replace(":10", ":12")})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/moves-3").returncode, 0)
        self.land(shifted, "preflight")
        self.land(shifted)
        done = self.settle(shifted, "fix/shifts-a-line")
        self.assertEqual(done.returncode, 0, done.stdout)

    def test_a_pull_request_retargeted_after_preflight_stops(self):
        where = self.branch("fix/retargeted", {"x.txt": "1\n"})
        self.land(where, "preflight")
        self.env["STUB_GH_BASE"] = "release/1"
        self.land(where)
        done = self.settle(where, "fix/retargeted")
        self.assertEqual(done.returncode, 7, done.stdout)
        self.assertIn("release/1", done.stdout)
        self.assertNotIn("x.txt", self.main_files())

    def test_a_runner_killed_after_the_merge_proves_it_on_the_next_turn(self):
        self.env["STUB_GH_KILL_AFTER_MERGE"] = "1"
        where = self.branch("fix/killed-after-merge", {"x.txt": "1\n"})
        self.land(where, "preflight")
        self.land(where)
        done = self.settle(where, "fix/killed-after-merge")
        self.assertEqual(done.returncode, 0, done.stdout)
        self.assertIn("x.txt", self.main_files())
        self.assertEqual(self.git(where, "ls-remote", "origin", "refs/heads/fix/killed-after-merge"), "")

    def test_main_moving_mid_gate_gates_again_against_it(self):
        once = os.path.join(self.root, "moved-once")
        self.mover()
        mover = self.branch("fix/moves-4", {"moved.txt": "1\n"})
        where = self.branch("fix/races-main", {"checks/test.sh": 'sh "$LAND_TEST_MOVER"\n'})
        self.env["LAND_TEST_MOVE_ONCE"] = once
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/moves-4").returncode, 0)
        self.land(where, "preflight")
        self.land(where)
        done = self.settle(where, "fix/races-main", timeout=90)
        self.assertEqual(done.returncode, 0, done.stdout)
        landed = self.outcome("fix/races-main")["merge_commit"]
        self.git(self.repo, "fetch", "--quiet", "origin")
        self.assertEqual(self.git(self.repo, "rev-parse", landed + "^1"),
                         self.outcome("fix/races-main")["gated_base"],
                         "the second gate's base is what it merged onto")

    def test_main_moving_every_round_gives_the_turn_up(self):
        self.mover()
        mover = self.branch("fix/moves-5", {"moved.txt": "1\n"})
        where = self.branch("fix/never-quiet", {"checks/test.sh": 'sh "$LAND_TEST_MOVER"\n'})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/moves-5").returncode, 0)
        self.land(where, "preflight")
        self.land(where)
        done = self.settle(where, "fix/never-quiet", timeout=120)
        self.assertEqual(done.returncode, 7, done.stdout)
        self.assertIn("moved during each of", done.stdout)

    def test_a_head_github_never_sees_stops_and_preflight_says_to_take_it(self):
        mover = self.branch("fix/moves-6", {"moved.txt": "1\n"})
        where = self.branch("fix/stale-head", {"x.txt": "1\n"})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/moves-6").returncode, 0)
        self.land(where, "preflight")
        self.env["STUB_GH_STALE_HEAD"] = "0" * 40
        self.env["ARMADA_LAND_HEAD_WAIT"] = "1"
        self.land(where)
        done = self.settle(where, "fix/stale-head")
        self.assertEqual(done.returncode, 7, done.stdout)
        self.assertIn("did not show", done.stdout)
        del self.env["STUB_GH_STALE_HEAD"]

        again = self.land(where, "preflight", check=False)
        self.assertEqual(again.returncode, 1)
        self.assertIn("git reset --hard", again.stderr)
        self.assertNotIn("push -u", again.stderr, "never force-push over a gated merge")

    def test_a_push_to_the_branch_mid_gate_stops_before_merging(self):
        gate = os.path.join(self.root, "gate-open")
        mover = self.branch("fix/moves-7", {"moved.txt": "1\n"})
        where = self.branch("fix/pushed-under", {"checks/test.sh": f'while [ ! -f {gate} ]; do sleep 0.1; done\n'})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/moves-7").returncode, 0)
        self.land(where, "preflight")
        self.land(where)
        deadline = time.monotonic() + 30
        while self.outcome("fix/pushed-under").get("state") != "gating" or "running test" not in self.outcome("fix/pushed-under")["detail"]:
            self.assertLess(time.monotonic(), deadline, "the Check never started")
            time.sleep(0.1)
        self.commit(where, {"late.txt": "written while it was gated\n"}, "more work")
        open(gate, "w").close()
        done = self.settle(where, "fix/pushed-under")
        self.assertEqual(done.returncode, 7, done.stdout)
        self.assertIn("while it was gated", done.stdout)
        self.assertNotIn("late.txt", self.main_files())

    def test_the_setup_default_says_what_the_manifest_requires(self):
        here = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        manifest = open(os.path.join(here, "armada.yml")).read()
        requires = manifest.split("setup:")[1].split("requires:")[1].split("seed:")[0]
        wanted = [line.strip("- \n") for line in requires.splitlines() if line.strip().startswith("-")]
        default = open(LAND).read().split('ARMADA_LAND_SETUP", "')[1].split('"')[0].split()
        self.assertEqual(default, wanted, "the copy of setup.requires in scripts/land has drifted")

    def test_the_manifest_gates_both_of_the_line_s_suites(self):
        """Asserted here because this file may name the agent harness's own
        directory, and the Rust tests under `crates/` may not."""
        here = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        manifest = open(os.path.join(here, "armada.yml")).read()
        scripts = manifest.split("scripts_test:")[1].split("hooks_test:")[0]
        hooks = manifest.split("hooks_test:")[1].split("format:")[0]
        self.assertIn("run: python3 scripts/test_land.py", scripts)
        self.assertIn('- "scripts/**"', scripts)
        self.assertIn("run: python3 .claude/hooks/test_guard_merge.py", hooks)
        self.assertIn('- ".claude/hooks/**"', hooks)
        self.assertTrue(os.path.exists(os.path.join(here, ".claude/hooks/test_guard_merge.py")))

    def test_a_check_whose_command_is_missing_stops_rather_than_reds(self):
        mover = self.branch("fix/moves-11", {"moved.txt": "1\n"})
        where = self.branch("fix/no-such-tool", {
            "checks/test.sh": 'echo "error: no such command: nextest" >&2\nexit 101\n',
            "wanted.txt": "this must not land\n",
        })
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/moves-11").returncode, 0)
        self.land(where, "preflight")
        self.land(where)
        done = self.settle(where, "fix/no-such-tool")
        self.assertEqual(done.returncode, 7, done.stdout)
        self.assertIn("nextest", done.stdout)
        self.assertIn("not on this", done.stdout)
        self.assertNotIn("wanted.txt", self.main_files(), "a tool nobody installed merges nothing")
        self.assertNotEqual(self.git(where, "ls-remote", "origin", "refs/heads/fix/no-such-tool"), "",
                            "nothing was merged, so the branch is still there")

    def test_a_killed_runners_gate_worktree_is_reclaimed(self):
        left = os.path.join(self.repo, ".armada", "gates", "left-behind")
        self.git(self.repo, "worktree", "add", "--quiet", "--detach", left, "HEAD")
        where = self.branch("fix/after-a-death", {"x.txt": "1\n"})
        self.land(where, "preflight")
        self.land(where)
        self.assertEqual(self.settle(where, "fix/after-a-death").returncode, 0)
        self.assertFalse(os.path.exists(left), "the next turn takes back what a dead runner left")

    def test_a_relative_binary_is_refused_rather_than_traced(self):
        where = self.branch("fix/relative", {"x.txt": "1\n"})
        self.env["ARMADA_LAND_ARMADA"] = "target/debug/armada"
        done = self.land(where, "preflight", check=False)
        self.assertEqual(done.returncode, 1)
        self.assertIn("absolute path", done.stderr)
        self.assertNotIn("Traceback", done.stderr)

    def test_an_entry_missing_its_place_does_not_stop_the_line(self):
        where = self.branch("fix/beside-a-bad-entry", {"x.txt": "1\n"})
        self.land(where, "preflight")
        os.makedirs(self.state_file("queue"), exist_ok=True)
        with open(self.state_file("queue", "half-written.json"), "w") as out:
            out.write('{"branch": "fix/half"}')
        self.land(where)
        self.assertEqual(self.settle(where, "fix/beside-a-bad-entry").returncode, 0)

    def test_a_red_branch_keeps_its_place(self):
        one = self.branch("fix/one", {"one.txt": "1\n"})
        two = self.branch("fix/two", {"two.txt": "2\n"})
        for wt, name in ((one, "fix/one"), (two, "fix/two")):
            self.land(wt, "preflight")
            self.land(wt)
        self.assertEqual(self.settle(one, "fix/one").returncode, 0)
        self.assertEqual(self.settle(two, "fix/two").returncode, 4)
        place = self.outcome("fix/two")["place"]
        self.git(two, "fetch", "--quiet", "origin")
        self.git(two, "merge", "--no-edit", "origin/main")
        self.commit(two, {"checks/test.sh": "exit 0\n"}, "make the combination pass")
        self.land(two, "preflight")
        self.land(two)
        self.assertEqual(self.outcome("fix/two")["place"], place)
        self.assertEqual(self.settle(two, "fix/two").returncode, 0)

    def test_both_sides_of_the_comparison_are_prepared_the_same_way(self):
        mover = self.branch("fix/moves-8", {"moved.txt": "1\n"})
        where = self.branch("fix/prepared", {"x.txt": "1\n"})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/moves-8").returncode, 0)
        self.land(where, "preflight")
        self.land(where)
        self.assertEqual(self.settle(where, "fix/prepared").returncode, 0)

        ran = [line.split() for line in open(self.env["LAND_TEST_EVIDENCE"]).read().splitlines()]
        foundations = [line for line in ran if line[0] == "foundations"]
        checks = [line for line in ran if line[0] == "check"]
        self.assertGreaterEqual(len(foundations), 2, "each turn reads the base's own run and its own tree's")
        for line in foundations:
            self.assertEqual(line[1:3], ["seed=yes", "setup=no"],
                             "both sides are seeded, and neither is installed into")
        self.assertTrue(checks and all(line[1:3] == ["seed=yes", "setup=yes"] for line in checks),
                        "a Check runs after setup")
        inside = os.path.join(os.path.realpath(self.repo), ".armada", "gates")
        for line in foundations + checks:
            self.assertTrue(line[3].startswith("at=" + inside), line)

    def test_a_check_that_writes_a_tracked_file_stops_the_turn(self):
        mover = self.branch("fix/moves-9", {"moved.txt": "1\n"})
        where = self.branch("fix/writes-in-the-gate", {"checks/test.sh": "echo written-by-a-check >> shared.txt\n"})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/moves-9").returncode, 0)
        self.land(where, "preflight")
        self.land(where)
        done = self.settle(where, "fix/writes-in-the-gate")
        self.assertEqual(done.returncode, 7, done.stdout)
        self.assertIn("the Checks left files", done.stdout)
        self.assertNotIn("checks", self.main_files() - {"checks"} or set())

    def test_a_second_instance_of_a_known_finding_is_new(self):
        known = "FAIL  no vendor literal outside adapters\n        missing: crates/a.rs:10 — `openai`\n"
        self.write(self.repo, {"foundations.txt": known})
        self.git(self.repo, "add", "-A")
        self.git(self.repo, "commit", "--quiet", "-m", "one violation on main")
        self.git(self.repo, "push", "--quiet", "origin", "main")
        mover = self.branch("fix/moves-10", {"moved.txt": "1\n"})
        second = self.branch("fix/one-more", {"foundations.txt": known + "        missing: crates/a.rs:80 — `openai`\n"})
        self.land(mover, "preflight")
        self.land(mover)
        self.assertEqual(self.settle(mover, "fix/moves-10").returncode, 0)
        self.land(second, "preflight")
        self.land(second)
        done = self.settle(second, "fix/one-more")
        self.assertEqual(done.returncode, 4, done.stdout)
        self.assertIn("crates/a.rs:80", done.stdout)

    def test_a_dirty_tree_or_a_missing_stamp_is_refused(self):
        where = self.branch("fix/dirty", {"x.txt": "1\n"})
        self.assertEqual(self.land(where, check=False).returncode, 1)
        self.write(where, {"stray.txt": "untracked\n"})
        done = self.land(where, "preflight", check=False)
        self.assertEqual(done.returncode, 1)
        self.assertIn("stray.txt", done.stderr)


if __name__ == "__main__":
    unittest.main()
