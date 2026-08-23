# v1 decommission — the record

**M0 step 2, run 2026-08-23.** v1's history is archived first (step 1: branch `v1-archive`,
tag `v1-final`, 586 commits, deletion-protected on `origin`). Nothing here was deleted; the
store was **moved**.

## The repo list, read out of `~/.armada` before the move

v1's own store is the only authoritative record of which repositories it touched. Assembled from
`manifest.db` (`workspaces` table), `machine.yml` (`fleet.carry`) and the `repo_root` field of all
127 job records under `jobs/`.

| Repository | Evidence | Jobs | Worktrees left behind |
|---|---|---|---|
| `~/Development/armada` | `manifest.db` workspace `c24a68b6`; `machine.yml` `fleet.carry`; 36 jobs | 36 | none — no `.git/worktrees` directory exists |
| `~/Development/chariot` | 1 job record | 1 | none of v1's — its two registrations are Claude Code's own, under `.claude/worktrees/`, and are live |
| `.../scratchpad/jobtest` (throwaway) | 10 job records | 10 | directory no longer exists |

v1's worktrees lived under `~/.armada/workspaces/<repo>/<name>`, not inside the source repos.
All three `workspaces/` directories (`armada`, `chariot`, `jobtest`) were already empty at the time
of the move; the seven worktree paths named in job records were gone.

## What was done

| Action | Result |
|---|---|
| `git worktree prune -v` in `~/Development/armada` | no output — nothing stale to prune |
| `git worktree prune -v` in `~/Development/chariot` | no output — nothing stale to prune |
| `mv ~/.armada ~/.armada.v1-backup` | 110M, 127 job records intact. Moved, not deleted |
| Unload v1's launchd agent | **no such agent exists** — see below |
| `rm ~/.cargo/bin/{armada,arm}` | removed on the owner's decision — not in the step, see below |

## What the step assumed and the machine did not have

**There is no launchd agent to unload.** `~/Library/LaunchAgents` holds seven plists, none
Armada's, and `launchctl list` matches nothing. `docs/HANDOVER.md` §2 states the Job daemon "is
enabled on the owner's machine"; it is not, and was not at decommission time. No v1 process was
running either.

**The v1 binaries were still installed, and the step does not mention them.**
`~/.cargo/bin/armada` and `~/.cargo/bin/arm` (17MB each, built 2026-08-18) were on `PATH`.
Running either one recreates `~/.armada` from scratch, which is the confusion step 2 exists to
prevent. The owner decided to remove both; they are rebuildable from `v1-final` at any time.
