# Reserved designs

Items Armada's own use has raised — a gap, a bug, a thing worth building next — recorded here
one file per item so each can carry a status and be referenced by id instead of by a `§` that
moves every time another is added. Formerly `PLAN.md`'s numbered subsections 15.3.1 through
15.3.9; see [`PLAN.md`](../PLAN.md) §15.3 for the inbox mechanism these build on.

Numbered in the order each was thought of. Numbers are never reused or reassigned.

**Four statuses, and the closing one depends on what was opened.** `RESERVED` is a design that
wants its own pass; `BUG` is a defect found in use. A `RESERVED` item closes as `BUILT` and a
`BUG` closes as `FIXED` — the words are not interchangeable, because *"we built the thing"* and
*"the thing was wrong and is not now"* are different claims and a reader scanning this table is
usually asking which one applies.

| id | title | status | module | hook |
|---|---|---|---|---|
| [001](001-raised-items-need-identity.md) | Raised items need identity | BUILT | helm | One id space over four origins — an inbox entry resolves under `failures show`, `fleet inbox` draws an `ID`, and `fleet answer <id>` acknowledges the row; **the acknowledgement vocabulary is the half deferred** |
| [002](002-tasks.md) | Tasks | BUILT | helm | `armada task` / `armada tasks` — capture a thing to do without spending anything on it yet, then start it as a Job |
| [003](003-bridge-command-centre.md) | The Bridge as the command centre | RESERVED | helm | One screen for Fleet, Manifest and Guild, not three |
| [004](004-guild-inventory.md) | Seeing what is in your guild | BUILT | guild | `armada guild ls` / `show` — no verb showed you what you had |
| [005](005-inbox-label-not-identity.md) | The inbox records a label, not an identity | FIXED | fleet | Inbox entries keyed by Job name collided; they carry the Job's uuid, close when it ends, and `ls` shows the id |
| [006](006-guild-has-no-way-to-learn.md) | The guild has no way to learn | BUILT | guild | `armada guild upgrade` — templates ship as a branch in the guild's own repo, so the upgrade is a `git merge` |
| [007](007-scanner-should-propose.md) | The scanner should propose, not only report | BUILT | manifest | `config scan` proposes what it can prove and a tick list writes it — **drift is deferred and is the one half not built** |
| [008](008-armada-injects-its-own-skills.md) | Armada injects its own skills | BUILT | cross-cutting | One skill, compiled in and appended to every Drone turn and every Helm launch, plus `fleet.propose` — a Drone raises what it learned instead of editing `armada.yml`; **a Drone still cannot read the manifest**, which is the half left open |
| [009](009-smaller-things-raised-in-use.md) | Smaller things raised in use, each with its reason | RESERVED | cross-cutting | Five small gaps — `doctor` health, `config scan`'s missing middle option, guild remote status, sync prompts, `--help` drift |
| [010](010-armada-records-its-own-failures.md) | Armada records its own failures | BUILT | cross-cutting | `armada failures` — a failure Armada reported was gone as soon as the scrollback was; navigable at a terminal, and a refusal Armada meant is not one |
| [011](011-what-a-drone-may-do-unattended.md) | What a Drone may do unattended | BUILT | fleet | Nothing ever granted a Drone permission, so every Job stalled on its first `git commit` |
| [012](012-a-drones-progress-through-its-workflow.md) | A Drone's progress through its workflow | BUILT | fleet | A Job stored the current step name and nothing else — no history, no timing, and no way to tell a Drone's *"done"* from a gate agreeing |
| [015](015-batch-a-job-from-many-entries.md) | One Job from many entries | RESERVED | helm | Select several entries and dispatch them as one Job, not one each |
| [013](013-secrets-must-resolve-before-the-run-detaches.md) | Secrets must resolve before the run detaches | BUILT | manifest | `secrets:` resolves above the `--detach` branch and the detached child reads its parent's answers off stdin — never a file, never Armada's own environment, never argv |
| [014](014-report-what-you-know-went-wrong.md) | Reporting what you know went wrong | BUILT | cross-cutting | `armada report` — a dry-run printed `CREATED` for work it had correctly not done and exited 0, so [010](010-armada-records-its-own-failures.md) could never hold it |
| [016](016-what-the-gate-cannot-prove.md) | What the gate cannot prove | RESERVED | fleet | The two refusals are closed — a gate spawns the Job it waits on — and the **narrowings stay**: `review_clean` proves a reviewer read the committed branch, never that its findings were clean |
| [017](017-what-you-have-not-tried-yet.md) | What you have not tried yet | BUILT | cross-cutting | `armada untried` — fifteen features landed in a day and nothing could say which had ever been run |
| [018](018-a-place-for-settings.md) | A place for settings | BUILT | cross-cutting | `armada settings` — read-only, reusing Manifest's, Helm's and Guild's own readers; **no writer**, which is the half that stays open |
| [019](019-the-brief-a-drone-reports-through.md) | The brief a Drone reports through | BUILT | fleet | `drone::BRIEF` — a Drone talks to Helm, not to you, so it is handed the tools' contract rather than your register; **it may name tools [011](011-what-a-drone-may-do-unattended.md)'s posture denies**, which is the risk left open |
| [020](020-the-tui-decided.md) | The TUI, decided | RESERVED | helm | Nine decisions from a design session after real use; **six of nine are built** — the Stop hook drives tick, the pane says what a Drone said, counts replaced the aggregate word, the window position leads the spend, and the table grew an id and a wide layout. **The three left are the ones that change a surface**: one signals listing, the menu, and the cmux handoff |
| [021](021-the-work-hierarchy.md) | The work hierarchy | RESERVED | cross-cutting | Fleet is a collection, the Job is the state machine, and the five things that lead to work are Signals |
| [022](022-docker-hygiene.md) | Armada cleans up after itself in Docker | BUILT | cross-cutting | 171 volumes and 12.0 GB, none of it Armada's — `doctor` reports the two shares apart, `armada manifest prune` can offer what is not Armada's, and a Job that **finishes** now releases what it holds |
| [023](023-status-shows-what-is-running.md) | Status shows what is running | FIXED | manifest | `status` enumerated the port registry, so six leaked process groups against a workspace that never claimed a block were invisible; it now enumerates the union, names what is provably gone, and reports undecided detached runs |
| [024](024-the-relay-does-not-fire.md) | The relay's watcher never ticks | FIXED | fleet | The hook fires and the watcher waited on **itself** — `$$`'s process group is the hook's, not the Drone's, so it ticked at `waited=0` every time; `$PPID` is the Drone, and Jobs now advance unattended |
| [025](025-a-drone-reached-outside-its-worktree.md) | A Drone reached outside its worktree | RESERVED | fleet | An unattended Drone ran `npx skills add -g` and installed third-party software on the operator's machine — **left open deliberately**: one incident is not a pattern, and the `HOME` form of the fix would move `~/.claude` out from under the Drone itself |
| [026](026-a-job-never-reaches-done.md) | What it takes for a Job to reach `DONE` | FIXED | fleet | `fleet answer` resumed the Drone instead of settling the gate, so an approval was read never — **fixed**. The headline claim was wrong: `design` and `plan` end at a human by design, and only [016](016-what-the-gate-cannot-prove.md) stands between a branch-ending workflow and `DONE` |
| [027](027-the-cli-under-a-users-hands.md) | The CLI under a user's hands | RESERVED | cross-cutting | A review of the whole CLI, driven rather than read — six things that stop somebody, led by a tick list whose printed instruction is false at a terminal |
| [028](028-specialised-drones.md) | Specialised Drones, and subagents a repository declares | RESERVED | cross-cutting | A repo declares its own subagents the way it declares skills, and Armada ships a fleet of specialised ones — a reviewer that cannot `Edit` is a mechanism where *"you are reviewing"* is only guidance |
| [029](029-a-job-of-its-own-done.md) | A Job says what done means | RESERVED | fleet | Done is the workflow's, so two Jobs on one workflow finish the same way however different the tasks were — and a prose goal self-assessed by a Drone is the false pass [PLAN.md](../PLAN.md) §14.3 exists to refuse |
| [030](030-armada-helps-write-the-prompt.md) | Armada helps write the prompt, and you approve it | RESERVED | cross-cutting | Classification already pays for a turn; a bad task is the most expensive failure Armada has. Approve, edit or skip — and no terminal means no rewrite |

**Armada's task system exists** ([002](002-tasks.md)), so these can become its first rows — one
file per row is what makes that a script rather than a rewrite. It is not run automatically:
`armada task` writes down what *you* decided, and a backlog that fills itself is one nobody
trusts.
