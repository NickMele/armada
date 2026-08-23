# v1's worktree handling

Source: `crates/fleet/src/worktree.rs` (866 lines, `git show v1-final:crates/fleet/src/worktree.rs`),
its callers in `crates/helm/src/verbs/fleet.rs` and `crates/fleet/src/land.rs`, and
`crates/fleet/src/home.rs` (path derivation). History: `git log v1-final -- crates/fleet/src/worktree.rs`
— 6 commits, one of them a real bug hit twice on a real machine (`a89ed78`).

**BLUF: the mechanisms are sound and mostly portable as-is; the policy layer around them —
where the path lives, who stores it — is what v1 got wrong and v2 has already fixed by moving
worktrees inside the repo.** Every function below builds argv and calls `ctx.run`, so nothing
here fights adoption into `git2` except that `git2` doesn't wrap `git worktree` — see the git2 note.

## Mechanisms: port / adapt / reject

| Fn (v1-final:worktree.rs) | Verdict | Reason |
|---|---|---|
| `add()` L46-89 | **Port** | `-b <branch>` is not optional — omitting it checks out an existing branch and interleaves two Jobs' commits. Named portable in the plan; keep the pre-flight `branch_exists` refusal (below) with it, not separately. |
| `add_detached()` L96-118 | **Port** | `--detach`, no branch — for a scratch re-run worktree that must never land in the `armada/` branch namespace. |
| `branch_for()` L20-23 | **Port** | Namespacing (`armada/<name>`) is what makes `delete_branch` safe to run at all — without it a Job named `main` could delete a person's branch. |
| `carry()` L130-153 + `copy_path()` L155-168 | **Adapt** | Plain filesystem copy, never `git add`s. Function itself trusts its input list; safety (no `..`, no absolute path) is checked once at the *read* site (`machine.rs::carry_for`, Lesc `escapes_root`), not here. Port that split as-is — checking at every call site is how the check gets forgotten once. |
| `remove()` L172-185 | **Port** | Always `--force`. Right for a human-invoked end (`kill`, `reap`) where a dirty tree is the ordinary case and the caller asked by name. |
| `holds_uncommitted_work()` L192-225 | **Port** | The *other* guard — the one that stops an automatic pass from calling `remove()`. Two design choices worth keeping exactly: untracked files count as work, and a `git status` that fails to answer (any error, not just "dirty") counts as holding work. Conservative in both directions on purpose. |
| `delete_branch()` L229-240 | **Port** | `-D` not `-d` — a Job's branch is never merged anywhere, so `-d` would refuse every time. Only ever called on the `armada/` namespace. |
| `is_registered()` L247-263 | **Reject as-is / re-derive if needed** | Dead code in v1 — built, tested, never called by any verb (`git grep is_registered` finds only its own definition and test). Don't port the unused function; if v2 needs "is this path still a real worktree," write it fresh against the actual need. |
| `branch_exists()` (private) L233-244 pattern | **Port** | `git rev-parse --verify --quiet refs/heads/<branch>`, failure = no. This is the pre-flight `add()` needs (below). |

## The one bug that took two nights: `a89ed78`

`git worktree add -b` on a branch name that already exists fails with git's own sentence —
*"could not create the worktree: Preparing worktree (new branch 'armada/column-order')"* — which
names the operation, not the reason, and suggests nothing. This collided with `kill --keep-branch`
(keep the branch, drop the tree) followed by a respawn under the same name: **recorded twice on a
real machine**, worked around by hand both times before it was fixed.

Fix: probe with `branch_exists` *before* calling `git worktree add`, refuse by name with both ways
out (rename, or `git branch -D` the old one). Test harness trap worth repeating: the probe and the
`branch_exists` *gate* used elsewhere issue byte-identical argv and differ only in `cwd` — a fake
`Run` that can't distinguish call sites by `cwd` breaks either every spawn or every gate test.

## The second trap: `carry()` and `git status`

The most-defended test in the file (`a_carried_file_stays_untracked_and_git_status_is_clean`,
comment: *"the test that matters most"*) runs **real `git`**, not a fake — because what's under
test is git's own opinion of the tree, which nothing in the module computes. `carry()` copies a
gitignored file (e.g. local secrets/config) into the worktree via plain filesystem copy; the trap
would be forgetting that the same `.gitignore` that hides it in the source repo is checked out
onto the new branch too, so it stays hidden in the worktree for free — but only if `carry` never
calls `git add`. One line of `git add` here would silently make every carried secret trackable.

## What changed underneath: path storage

v1 kept worktrees at `~/.armada/workspaces/<repo>/<name>` (`home.rs::worktree()`), **outside**
the source repo, and stored the resulting path as a string field on the `Job` record
(`record.worktree`), converted to `~`-form for display (`home::tilde`) and re-expanded
(`place.expand`) everywhere it was used. `docs/v1-decommission.md` records this store was already
moved once, off-disk, before v1 was archived.

v2's rule — `<repo>/.armada/worktrees/<job-id>`, **derived, not stored, not configurable** — is a
strictly better version of the same idea, not a variant needing its own justification:

- A path that's a pure function of `(repo, job-id)` can't drift from what the record says, which
  is a whole failure class v1's `record.worktree` field could hit (a record that lies about its
  own worktree — the same shape of bug `release_on_finish`'s comment calls out for `RUNNING`
  Jobs with dead Drones) and doesn't need `place.expand`/`place.shown`/`tilde` machinery at all.
- **New surface v1 never had**: the worktree now lives *inside* the repo it was cloned from, so
  the outer checkout's own `git status`/`git add -A` can see it as an untracked directory unless
  `.armada/worktrees/` is excluded (`.gitignore` or `--exclude`). v1's worktrees, living entirely
  outside any repo, could never collide with a repo's own tree this way — this is new risk to
  carry, not a solved problem to adopt.
- `carry()`'s untracked-file guarantee (above) still holds regardless of where the worktree sits,
  since it depends only on the *worktree's own* `.gitignore`, not on nesting — but the nesting
  itself needs its own coverage now (see test cases).

## `git2`

v1 shelled out to `git` for every operation here (argv + `ctx.run`, deliberately, "so a test
asserts the exact command"). v2 uses `git2`. Nothing in the *mechanism* list above resists that
translation — `add`/`add_detached`/`remove`/`delete_branch` are thin argv wrappers with a git2
equivalent (`Repository::worktree`, `Worktree::prune`, `Branch::delete`). The one place to check
directly against `git2`'s actual behavior rather than assume: `holds_uncommitted_work` needs
`--untracked-files=normal` semantics (untracked counts, ignored-directory contents don't) —
confirm `git2::Repository::statuses` with the matching `StatusOptions` produces the same
untracked/ignored split before trusting it for the same guard.

## Test cases the failure modes imply

| Case | Source |
|---|---|
| `add()` on a branch name that already exists is refused before any worktree is created, and names the branch + both remedies | `a89ed78` |
| `add()` always passes `-b`; never checks out an existing branch by name collision | doc comment, top of file |
| `add_detached()` never lands on any branch, including `armada/` | daemon re-run worktree contract |
| `carry()` of a gitignored path leaves `git status` clean in the destination worktree (real git) | "the test that matters most" |
| `carry()` silently skips a declared path absent from the source (fresh clone is not an error) | existing v1 test |
| `carry()` refuses a path with `..`, a leading `/`, or a `.` segment — checked at read time, not copy time | `machine.rs::escapes_root` |
| `holds_uncommitted_work()` is `true` for an untracked file, not just a modified one | existing v1 test |
| `holds_uncommitted_work()` is `true` when `git status` fails or git is missing — never treated as "clean" | existing v1 test; same rule as the reaper's `ENOENT`-only removal |
| `remove()` always forces, regardless of dirty state — used only where a human/automatic-but-authorized caller already decided | contract split vs. `holds_uncommitted_work` |
| **New for v2**: `.armada/worktrees/<job-id>` does not appear in `git status --porcelain` of the *outer* repo checkout | nesting risk, not present in v1 |
| **New for v2**: an `interrupted` Job's worktree is never selected by whatever v2's reap/sweep equivalent is — v1 had no `interrupted` state to test against; this is a new state needing its own coverage, not an inherited one | task framing; v1's `JobState` has no `Interrupted` variant (`Queued/Running/Paused/Stalled/Silent/Blocked/...`) |
| Worktree removal on Job finish only when the tree is clean; a dirty finish keeps the directory and says so, never removed silently | `release_on_finish` design comment, v1 |
| A Job's worktree survives a Drone dying/exiting; only Job-level end (`kill`, `reap`, finish-when-clean) removes it | v1's `pause`/`resume` already draws this line for the Drone-process case — the same boundary the task's "worktree belongs to the Job" rule states, so this is precedent to point to, not a new argument |

## Owner

Findings only — no `crates/` or `apps/` changes made or recommended by this note directly; a
v2 implementer acts on the port/adapt/reject table above.

## Correction, added by the step that commissioned this note

The note flags that worktrees nesting inside the repo let the outer checkout's
own `git status` see `.armada/worktrees/`, and records no v2 answer. There is
one: the System Architecture accepts exactly this cost and states the mitigation
— **Armada adds `.armada/` to the repo's `.gitignore` during Manifest setup**,
alongside the second accepted cost, that Fleet sweeps worktrees for terminal Jobs
past retention on startup.

So it is a known cost with a decided answer rather than an open risk. What the
note gets right is that **v1 never faced it**, so there is no v1 evidence for
whether that mitigation is sufficient — and the test case it proposes is worth
writing precisely because the design's answer has never been exercised.
