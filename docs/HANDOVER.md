# Handover — what one orchestrator learned before a real Helm took over

**Written 2026-08-17, at the end of the session that ran the fleet by hand while the fleet was being
made able to run itself.** The owner is replacing this orchestrator with `armada helm`. Everything
below is what the next one would otherwise have to rediscover.

**This file is a handover, not a design.** Nothing here overrides
[`ARCHITECTURE.md`](ARCHITECTURE.md), [`PLAN.md`](PLAN.md) or the reserved designs; where it
disagrees with them, they win and this file is stale. Delete it once its contents have been absorbed
or have stopped being true.

## 1. Read these first, in this order

| | |
|---|---|
| [`glossary.md`](glossary.md) | The vocabulary is fixed. A Job is durable and *is* the state machine; a Drone is one ephemeral `--print` exchange. Getting these two confused is the root of most of what went wrong below. |
| [`traps.md`](traps.md) | Measured behaviour. Two entries were added this session and both cost real time to find. |
| [`reserved/034-the-job-daemon-lands-the-work.md`](reserved/034-the-job-daemon-lands-the-work.md) | The owner's largest decision this session, and the one in flight. |
| `armada tasks` and `armada failures` | **The real backlog.** Not this file. 14 tasks and 4 failures are open, each written to be read cold. |

## 2. What is in flight right now

**Job `build-034`** (`7ed8e136`) with its plan sub-Job `43ec9b34`, on task `56bb1535` — building
[`034`](reserved/034-the-job-daemon-lands-the-work.md) stage one, the daemon's mechanical half. It was
left running deliberately: killing it would have destroyed work in progress. Its monitors were stopped,
so **nothing is watching it.** Check it, and note that it will reach `land`, leave a branch, and stop —
because nothing merges yet, which is what it is being built to fix.

**Branch `armada/wire-bridge`** — 2 commits, the Bridge's MANIFEST panel, `DONE` and unmerged. It
merges cleanly into `main` and passes 5/5 *after* merging; that was verified by staging the merge on a
throwaway branch, running the checks, and discarding it. **It is waiting on a person by choice**, not
by oversight: the owner's instruction was to stop landing work on Jobs' behalf.

**Eighteen other unmerged branches.** Task `2829154e` triages them. Two matter: `armada/command-centre`
(5 commits) and `armada/job-drives-the-drone` (8) are both `DONE` Jobs whose features reached `main` by
another route, so their branches are *behind* `main` and merging them now would revert later work —
task `eefc8a26` covers what is worth salvaging, which is documentation rather than code.

## 3. The one thing to do differently

**Stop doing the fleet's work for it.** This is the owner's own correction, and it is the most
important sentence in this file:

> *"Why are you committing on behalf of jobs? That's it. We need a daemon."*

Over one session this orchestrator hand-landed three Jobs' work, ticked stalled Jobs by hand, and
merged a Job's uncommitted implementation itself. Each was locally defensible — the alternative was
losing green work — and the aggregate was that **every gap in the fleet stayed invisible because
something else was covering it.** The daemon exists as a design because the covering finally stopped.

When a Job cannot finish something, the useful act is to make the Job able to, or to write down
precisely why it cannot. Doing it by hand produces a working repository and no progress.

## 4. Patterns that cost the most time

### A fixed path shared by every writer is a race, twice over

`Store::save` staged the Job index at `<uuid>.json.new` — one path per Job, shared by every process
writing it. Two concurrent saves both wrote it, the first `rename` moved it, the second got `ENOENT`
and reported *"could not write the Job index"* naming the **final** path, which was the one place that
was fine. Reproduced 19 times in 100 saves.

The identical bug was then found in `doctor`'s probe: `probe-<PROBE_SESSION>.json`, where
`PROBE_SESSION` is a **constant**, written and deleted by every `doctor` on the machine — and the
Bridge runs `doctor` on every redraw. It surfaced as a test that failed three times under load and
passed alone every time, which reads as flakiness and is not.

**Both fixes are the same: put the writer's identity in the name.** When something intermittent
involves a temp file, look at whether its name is unique before looking anywhere else.

### An error that names the wrong thing sends you to the wrong place

Three separate defects this session hid behind a message about the wrong subject:

- `ENOENT` on the *final* path when the *staging* path was the problem.
- *"could not read the Job index"* with `class: environment` for a Job that simply did not exist,
  advising a retry that could not help. It is `bad_invocation` now.
- `armada:fmt` reported as a race between a fix and a check, because a `--fix` run reports `PASS`
  whether or not it changed a file — so *"PASS with no file changed"* was an inference, not an
  observation, and the real explanation was a dogfood test leaving a run directory in the tree.

**When a message names a subject, check that the subject is where the failure is.**

### A ceiling is a checkpoint, and prose in a brief is not a mechanism

`Budget.cost_usd` is evaluated when a pass *looks* at a Job, and a pass cannot look mid-exchange. A
Job with a `$10` ceiling paused at `$20.48`. Documented on the field rather than narrowed, because
closing the window means killing work in progress to save money already spent.

Separately: fifteen inbox entries were the string *"does this look right to you?"* and nothing else,
while both the brief **and** the tool's own schema documentation already said to write the question in
full. **Two statements of a rule in the two places a model reads produced fifteen identical
failures.** The fix was a second required field the Drone cannot omit. Prefer a shape that cannot be
got wrong over a sentence that asks.

### Approving a long document is a claim about all of it

A 354-line plan's fourth open question said *"this needs the owner's confirmation, not an
assumption"*. The approver read questions one to three, said *"approved, implement it as written"*, and
never reached the fourth. The implement step then did what the plan said — correctly — and the decision
was taken by nobody.

**This was then misfiled as a Drone exceeding its scope** (failure `4bcbbd35`, cleared; correction
`c99e814d`). The reviewer misread it the same way, because a review step cannot see the inbox where
the approval happened. Task `69b684c0` is the real fix: the approval question should be built by the
**Job** from the plan, not written by the Drone — a Drone asked *"does this look right"* about a
document the Job can read is being asked for something the Job already has.

## 5. Decisions the owner took this session

Each of these is written up where it belongs; they are listed here so a new orchestrator does not
reopen them.

| Decision | Where |
|---|---|
| A daemon lands work: push, PR, merge on green CI, re-run on `main`, reap, mark `main` moved | [`034`](reserved/034-the-job-daemon-lands-the-work.md) |
| Auto-merge is **per repository** (`fleet.land.merge` in `armada.yml`), defaulting to `never` | `034` §6.4 |
| The daemon's audit trail lives **on the Job**, so the Bridge stays a renderer over Fleet | `034` §6.5 |
| The daemon's spend limit has three spellings — `stop_on_warning`, `stop_at_percent`, `daily_usd` — because one machine is billed by plan utilisation and another by dollars | `034` §6.1 |
| `carry` (untracked local config into a worktree) went to `machine.yml`, **not** `armada.yml` | commit `5296190` |
| Ceilings are cost, wall clock, and consecutive failed gates at one step. `iterations` was deleted | `workflow.rs` |
| The Bridge builds `App` once at `watch()` entry | built; landing tracked as task `dd81460f` |

**The `carry` and `merge` decisions look contradictory and are not.** The test is whether the fact is
about *this machine* or about *the thing itself*: a gitignored local path exists on one checkout, so it
is machine-scoped; "this repository's PRs may not be merged without a human" is true in every clone, so
it belongs with the repository.

## 6. Working habits worth keeping

- **`arm manifest check --fix` runs each check's declared `fix:`.** Do not reach for `cargo fmt`
  directly. The owner corrected this twice.
- **Golden fixtures have no update flag, deliberately.** Read every `.actual` by hand before moving it
  into place. A golden proves the bytes match what was reviewed; it does **not** prove they fit a
  terminal, which is why there is now a separate width assertion — added because the first nesting
  prefix tried took `fleet ls` from 80 columns to 87, and a golden alone would have recorded that as
  the new truth.
- **Editing a skill in `~/.armada/guild/` changes nothing until `arm guild project` runs.** A Drone
  reads `~/.claude/`. A `review_clean` fix was inert for hours because of this, while `armada doctor`
  printed `STALE ~/.claude` in every diagnostic. `spawn` now refuses a stale projection.
- **The templates in `templates/guild/` are the shipped default, not the owner's guild.** Changes there
  reach existing guilds through `armada guild upgrade`.
- **A doc comment and its item must stay adjacent.** Anchoring a text insertion on `pub fn X(` and
  inserting *before* it splices code between a doc comment and its function. This happened six times
  in one session; the compiler caught every one, and it is still six wasted cycles.

## 7. What is still owed, in the owner's own priority order

1. **`034` stage one** — in flight as `build-034`. Stage two (anything that resumes a Drone, and the
   rebase) waits on the spend limit being built.
2. **Land `armada/wire-bridge`, or decide not to** — task `dd81460f`, which carries the one thing worth weighing first.
3. **`arm failures`** — 4 open. `c6c1dd80` (the `review_clean` gate's structural half) is the oldest
   real defect; `599657a8` needs the owner's decision on whether `config scan` may propose *actions*
   as well as lines, and its diagnosis is complete in task `183ae03b`.
4. **`arm tasks`** — 14 open, several of them decisions rather than work: `eefc8a26`, `2829154e`,
   `9cc81375`, `183ae03b`.

**Nothing in this file is a substitute for reading those two listings.** They are written to be read
cold, each with its measurement in it, which is the one habit worth inheriting.
