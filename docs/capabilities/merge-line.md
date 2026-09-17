---
capability: merge-line
issue: 1315
milestone: Throughput
---

# Merges take turns, and each one is checked against the main it lands on

Two branches can each pass every Check, merge a minute apart, and leave `main`
red. Each one's Checks ran against a `main` that had moved by the time it
merged.

**Merges wait in line; work does not.** Branches are worked in parallel, and
write-scope overlap stays a warning ([Fleet](../concepts/fleet.md),
*Write-scope overlap*). Only the step onto `main` takes turns.

`scripts/land` is the local stand-in, for agents working this repository
outside Fleet. It is built so each part has a named home in Fleet, listed under
*Where each part goes in Fleet*. How to run it is
`docs/practices/running-locally.md`, *Landing a branch*.

## What holds

| Rule | Held by |
|---|---|
| Nothing reruns when `main` has not moved | `main` is an ancestor of the branch head |
| When it has moved, `main` is merged in first | A throwaway detached worktree at the branch head |
| A Check reruns when its `when:` matches both sides | `armada covers`, asked once per side |
| `verify-foundations` reruns, read against `main` | Only a failing line `main` lacks is red |
| The merge goes through GitHub, pinned to the gated commit | `gh pr merge --merge --match-head-commit` |
| `main` is read again right before merging | `git ls-remote`; a move gates again |
| An ungated combination landing is said | Merge commit's first parent against the gated base |
| An agent still asks the owner before landing | The agent's own brief; nothing here asks |

## One turn

```
scripts/land preflight    clean tree, pushed, PR open -> stamp HEAD^{tree}
scripts/land              stamp matches -> queue entry -> runner started if none -> returns
                                                  |
runner (holds flock) -----------------------------+
  read PR and remote head
  main an ancestor of head? -- yes --------------------------------------+
        | no                                                             |
  worktree add --detach <head>; git merge <main>                         |
        | conflict only in generated files -> regenerate, commit         |
        | any other conflict -> outcome conflict (keeps its place)       |
  covers(landed on main) ∩ covers(branch changed) -> armada check each   |
  verify-foundations: new FAIL / missing: lines vs main's own run        |
        | red -> outcome red, nothing pushed                             |
  push merge commit to the branch; wait for GitHub to see it             |
        v                                                                v
  ls-remote main == gated base?  -- no -> gate again (bounded rounds)
        | yes
  gh pr merge <pr> --merge --match-head-commit <sha>
  gh pr view --json state,mergeCommit; mergeCommit^1 == gated base?
        | no -> outcome ungated          | yes -> outcome landed
  git push origin --delete <branch>; print worktree cleanup commands
```

## The line and the turn

- **The turn is an `fcntl.flock` on `runner.lock`.** The kernel releases it when the holder dies, so there is no timeout and no eviction.
- **The runner is detached with `start_new_session=True`.** The Bash tool kills a command at 600 s; the runner outlives that call.
- **A dead runner's entry stays queued.** The next `scripts/land` or `--status` finds the lock free and starts a runner, which retakes the turn.
- **An entry is keyed by a hash of its branch** and carries the name, because branch names hold `/`.
- **Every finished turn removes its entry**, whatever the outcome.
- **A conflict keeps its place.** Resubmitted, the entry reuses the place its conflict outcome recorded.
- **State lives under the common git directory**, in `armada-land/`, so every worktree of one clone shares one line.

## Choosing what reruns

**Which Checks a set of paths hits is one answer, shared with Fleet's gate.** `armada covers` reads paths on stdin and asks each Check's `covers`, which calls `Covers::reach` in `crates/core-model/src/job/covers.rs`. `ResolvedCheck::covers`, which the gate's skip decision asks, calls the same function.

A Check reruns when it covers what landed on `main` since the merge base **and** what the branch changed. A Check with no `when:` covers everything, so `build`, `test` and `format` rerun on every moved `main`.

**File overlap alone would miss cross-file breakage.** A type changed in one crate breaks a caller in another file, and both sides still hit `test`.

## The merge and the proof

- **`--merge`, never a rebase or a squash.** Rebase-merging rewrites commit ids, so no check for "my commit reached `main`" ever ends.
- **The merged-in commit is pushed to the branch before merging.** That makes the PR head the exact tree the Checks ran on, which `--match-head-commit` then pins.
- **The gate runs in a throwaway worktree**, never the agent's own tree. A clean tree is required at preflight and at land, and the stamp is the tree id.
- **The runner removes only its own gate worktrees**, under `armada-land/gates/`. An agent's worktree is never removed; the outcome prints the `agent-worktrees` cleanup commands.
- **The remote branch is deleted with `git push --delete`** after the merge is confirmed. `gh pr merge --delete-branch` fails inside a worktree.

## Where each part goes in Fleet

| `scripts/land` | Fleet | Notes |
|---|---|---|
| The line and the turn | The sweep, and the places line | 1 |
| `armada covers` | `ResolvedCheck::covers`, already shared | 2 |
| Merging `main` in | `crates/adapters/src/merging_in.rs` | 3 |
| Rerunning the Checks | A gate run over the merged worktree | 4 |
| The merge | `crates/adapters/src/landing.rs` | 5 |
| First parent against the gated base | Proving what merged | 6 |
| Outcome file and `--status` | The Job record, served on detail | 7 |

1. `docs/concepts/fleet.md`, *Checks share one limit*. A press to merge joins a line the sweep turns, one landing per repository at a time.
2. Fleet needs no port for this part.
3. It already merges and never rebases, and leaves conflict markers for a Drone to clear.
4. The reruns take places like any other Check run.
5. It merges with `--merge` today. `--match-head-commit` and the base reread are what it lacks.
6. `docs/concepts/manifest.md`, *Proving what merged*. The first-parent comparison belongs beside `after_merge`.
7. An outcome becomes a Job event and a log line.

## What does not port

| Piece | Why it stays here |
|---|---|
| `flock` across processes | Fleet is one process; its turn is in-process state |
| The detached runner | Fleet already outlives the caller |
| `ARMADA_LAND_SETUP` and `ARMADA_LAND_SEED` | A second copy of `setup:`, which Fleet reads from the Manifest |
| The `GENERATED by` header as a regeneration recipe | Fleet leaves conflicts to a Drone |
| Printing cleanup commands | `armada clean` gives a Job's worktrees back itself |
| The installed `armada` binary gating | Fleet gates with its own build |

**Gates run the `armada` on `PATH`, not one built from the gated tree.** `armada check` only resolves a name in `armada.yml` and spawns its command, so the binary needs only to read the file. A change to how `armada.yml` is read is the one case it gets wrong.

## The guard, not yet built

**A hook refuses `gh pr merge` and a push to `main`, and names `scripts/land`.** Without it an agent that merges by hand bypasses the line, and the first-parent check reports the result as ungated after the fact.

It belongs beside `.claude/hooks/guard_write.py`, and its settings entry is a `PreToolUse` matcher on Bash. `scripts/land`'s own `gh pr merge` runs in the detached runner rather than through the Bash tool, so the hook does not see it.

## What it depends on

- `concepts/fleet.md` — *Write-scope overlap*, *Catching a branch up*, and what Fleet knows after a merge.
- `concepts/manifest.md` — *Which paths a Check covers* and *Proving what merged*.
- `practices/running-locally.md` — *Landing a branch*, how to run it and read its outcomes.

## Open questions

- **[merge-line-new-warnings]** Should a `verify-foundations` warning the merged tree has and `main` lacks stop a merge? `scripts/land` reads only new `FAIL` and `missing:` lines as red, because the summary line's counts change with every merge and a warning does not fail `main`. Counting new warnings would stop merges that add a legitimate known gap.
- **[merge-line-red-keeps-place]** Should a branch that goes red keep its place in line, as a conflict does? Today it rejoins at the back. A red is usually the branch's own fix to make, but it can also be `main` breaking the combination.
- **[merge-line-no-when-reruns]** Should Checks with no `when:` rerun on every moved `main`? `build`, `test` and `format` declare none, so today any movement reruns all three, including a docs-only merge. Narrowing them needs a `when:` on each in `armada.yml`.
