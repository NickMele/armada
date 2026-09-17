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
| `verify-foundations` runs on every turn | A throwaway worktree at the commit being merged |
| No Check reruns when `main` has not moved | `main` is an ancestor of the branch head |
| When it has moved, `main` is merged in first | A throwaway detached worktree at the branch head |
| A Check reruns when its `when:` matches either side | `armada covers`, over both sets of paths |
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
  worktree add --detach <head>
  main moved? -- yes -> git merge <main>
        |            | conflict only in generated files -> regenerate, commit
        |            | any other conflict -> outcome conflict (keeps its place)
  seed: cp -c the build directories in
  verify-foundations: new FAIL / missing: lines vs main's own run
  main moved? -- yes -> covers(landed + changed) -> setup -> armada check each
        | red -> outcome red, nothing pushed
        | main moved -> push the merge commit to the branch, wait for GitHub
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
- **A conflict and a red keep their place.** Resubmitted, the entry reuses the place the outcome recorded: the wait was already served.
- **A hung Check holds the turn until somebody kills the runner.** A timeout was ruled out, because a cold build plus the app suite runs past any fixed one and evicting a holder that is still working puts two merges in flight.
- **State lives under the common git directory**, in `armada-land/`, so every worktree of one clone shares one line.
- **The worktrees the Checks run in live under `.armada/gates/`**, inside the repository, beside the ones Fleet cuts for a Job. A checkout outside the repository directory is not somewhere this project's tooling runs: Vite refuses to serve a `node_modules` outside its allow list and `tsc` cannot name a type through one, so three Bridge Checks failed there for reasons that had nothing to do with the branch. `gates/` rather than `worktrees/` keeps them out of what `armada clean` accounts for, and `.armada/*` is already ignored.
- **A gate worktree left by a killed runner is taken back by the next turn.** Holding the lock means nothing else is gating, so whatever is still under `.armada/gates/` belongs to a runner that died.

## Choosing what reruns

**Which Checks a set of paths hits is one answer, shared with Fleet's gate.** `armada covers` reads paths on stdin and asks each Check's `covers`, which calls `Covers::reach` in `crates/core-model/src/job/covers.rs`. `ResolvedCheck::covers`, which the gate's skip decision asks, calls the same function.

**A Check reruns when it covers what landed on `main`, or what the branch changed, or both.** Either side, not both: the pair most likely to break only in combination is a Rust change landing on the base against a branch's TypeScript, where the generated types meet, and asking for both sides skips exactly that.

**Every Check in this repository declares `when:`, including `build`, `test` and `format`.** They name what their commands read rather than what they are about — the workspace, the lockfile, `.cargo/`, `protocol-version.toml`, the shipped workflow definitions, `armada.yml` itself, and for `test` the Bridge tree that `xtask`'s own tests read. A change to the documents alone now hits no Check at all.

**File overlap alone would miss cross-file breakage.** A type changed in one crate breaks a caller in another file, and both sides still hit `test`.

**The gate runs every turn and the Checks only on a moved one.** `verify-foundations` reads the tree, takes seconds, and an unmoved turn is the one way a branch can break it for everybody behind it — with the line the only route to the fix. A Check reads the combination, takes minutes, and there is no combination to read until the base has moved.

**So the line gates the combination, and trusts the agent for the branch's own Checks.** `work-issue` step 4 is where a branch is measured on its own, and preflight stamps the tree it was measured on.

**How a `verify-foundations` run is read:**

| Read | Why |
|---|---|
| Only `FAIL` and `missing:` lines | A warning does not fail `main` either |
| Line numbers normalised out of the subject | A line inserted above an old failure renumbers it |
| Findings counted, not collected into a set | A second violation of one rule in one file reads like the first |
| A non-zero exit naming no failing rule is red | A branch that breaks `xtask` prints one `error[E0433]` and would be gated on nothing |
| A Check whose command is not installed stops the turn | A missing tool is not the branch breaking `main` |
| The same on `main`'s own run, which stops the turn | There is nothing to compare against |
| `main`'s run cached per commit, only once read as a report | A killed run cached empty makes every branch after it red |
| One report carries the gate and the Checks together | An agent reads everything wrong once, not twice |
| A `main` that cannot run it stops every turn, saying so | The branch behind it is not the one to fix |

## What a turn prepares, and in which order

**The build directories are cloned into both worktrees, and nothing else runs before `verify-foundations`.** `setup.requires` — `pnpm install` and the browser download — runs only where a Check is about to, after the comparison is read.

```
merge main in -> seed (cp -c) -> verify-foundations -> setup, if a Check reruns -> the Checks
```

**The two sides of the comparison are then identical by construction**, rather than by two code paths being kept in step. What makes it safe to read `verify-foundations` in a tree with no `node_modules` and no built bundle:

- A missing bundle is a warning rather than a failure, and warnings are outside the comparison. `xtask/src/rules_bundled.rs` says so about itself.
- The one rule that shells out to `node` runs the codegen script directly, with no installed dependencies. `xtask/src/rules_vocabulary.rs`.
- Measured: a worktree with neither produced no new failing line against the main checkout's own run.

**What it gives up:** a rule that did read build output would be blind on both sides. The bundle rule already declares a warning for that reason, and a rule that wanted more would have to say so.

**The seed stays on both sides, and a clone that fails stops the turn.** It is an APFS clone and it is what keeps `xtask` from cold-building — but a clone that worked in one tree and not the other would prepare the two sides differently, silently, which is the failure this order exists to remove.

**`main`'s own run is cached by its commit, and a commit does not carry the machine.** A cached result was taken whenever it was taken, with whatever was installed then, so a machine that changed underneath is compared against a reading from before it did. Deleting `armada-land/foundations/` is how that is thrown away.

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
| The gate worktree under `.armada/gates/` | `.armada/worktrees/<handle>`, cut by `Vcs` | 8 |
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
8. Same directory, same reason: a Job's Checks run inside the repository because that is where this project's tooling works.

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

## The guard

**`.claude/hooks/guard_merge.py` refuses `gh pr merge` and any `git push` whose destination is `main`**, including `--delete main`, and splits a compound command so `cd x && git push origin main` is caught too. Its refusal names `scripts/land preflight`, `scripts/land`, `scripts/land --status` and this page. `.claude/settings.json` registers it as a `PreToolUse` matcher on Bash.

**Both halves are gated by the Manifest.** `scripts_test` runs the script's suite when anything under `scripts/` or `armada.yml` changes, and `hooks_test` runs the hook's when anything under `.claude/hooks/` does — two Checks rather than one, because a `run` gets no shell to chain them with and because the two are read by different changes.

**It cannot see the line's own merge.** That runs in the detached runner, outside the Bash tool. An agent who goes around it lands a combination nothing checked, and the first-parent comparison is what says so afterwards.

## What it depends on

- `concepts/fleet.md` — *Write-scope overlap*, *Catching a branch up*, and what Fleet knows after a merge.
- `concepts/manifest.md` — *Which paths a Check covers* and *Proving what merged*.
- `practices/running-locally.md` — *Landing a branch*, how to run it and read its outcomes.

