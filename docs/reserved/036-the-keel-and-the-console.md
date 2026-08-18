---
id: 036
title: The keel, and the console
status: RESERVED
module: cross-cutting
raised: the owner asking where to go from here, 2026-08-17
---

# 036 — The keel, and the console

**What this settles.** Four decisions taken in one session, after the owner said the project had
become *"a fragmentation of various ideas, and a large Rust code base that is now slow to compile"*
and asked what the plan was to get out of it. Each is his answer with the trade stated. Together
they name **one keel** — the thing every other part of Armada now serves — and retire the surface
that four earlier designs were building toward.

This document is the reasoning. [`PLAN.md`](../PLAN.md) is the order to do it in.

## How this was found

Not from reading the code. Three things were measured, and each one moved the answer.

**The tree.** Five crates, 145,819 lines of Rust, 2,181 tests, 586 commits — all of them inside
fourteen days. The presentation layer is **ten times the size of the engine it presents**:

| crate | lines | what it is |
|---|---|---|
| `helm` | 77,906 | CLI, args, render, Bridge |
| `core` | 39,868 | envelope, failure, fleet types, schedule |
| `guild` | 9,183 | cross-machine sync |
| `manifest` | 8,090 | `armada.yml`, checks |
| `fleet` | 7,704 | jobs, drones, worktrees, daemon, land |

`fleet` is the thing the owner said *"has never really worked end to end"*. It is five per cent of
the code. `render.rs` alone is 8,348 lines.

**The gates.** Every predicate in [`workflow.rs`](../../crates/core/src/fleet/workflow.rs)'s
`Predicate` is a **state** predicate: it reads the world after a turn and asks whether it is in a
given shape. None reads what the turn *changed*. Across the five shipped workflows and their
fifteen steps:

| predicate | steps | what passes it that should not |
|---|---|---|
| `always` | 2 | anything, including nothing |
| `artifact_exists` | 3 | an empty file; a file the previous step wrote |
| `check_passes` | 2 | a Drone that changed nothing, when the suite was already green |
| `review_clean` | 2 | resolves to a child Job running `review.yml`, whose only step gates on `artifact_exists` — **satisfied by a reviewer writing a file nothing reads** |
| `human_approves` | 3 | nothing. No surface asks the question; `workflow.rs` says *"a future surface"* |
| `branch_exists` / `pr_open` / `pr_merged` | 2 | git state, not achievement |
| `failing_test_exists` | 1 | `assert!(false)` in a test of the right name |

**The corpus.** `~/.armada/failures.jsonl`, `inbox.jsonl` and `untried.jsonl` hold 77 entries
recorded in use. Nine of the eleven readable failures are the same complaint in different words —
*the Bridge freezes · resizing breaks the layout · the legend is illegible · the columns
flip-flop · "there are conflicts, I have no idea what the conflicts are"*. **The recorded pain is
the surface, not the engine.** That is the finding that reordered everything below.

## 1 · The keel is the Job/Drone verification loop

Armada is four products sharing a binary: a verification loop, a triage surface, a manifest, and a
guild. Asked which one he would still install on both machines if the other three were deleted, the
owner chose **the loop** — a deterministic daemon that holds the workflow, runs the checks, and
tells a Drone pass or fail with evidence, so the Drone never has to know how to verify itself.

It is the only genuinely novel part, and it is the part that has never run once.

**What follows from calling it the keel:** everything else in the tree either serves it or leaves
the build. Not deleted — `guild` and `helm` stay in the repository and return by earning it. The
rule is in [`PLAN.md`](../PLAN.md).

## 2 · Gates are two-tier, and the judge is a veto

The owner's objection to the table above closed the only predicate that survived it:

> *"even a failing test existing is not necessarily a hard thing to fake. Anybody can write a test
> that just fails. The question is, is it the proper test with the proper assertions?"*

He is right, and it is not a gap in the predicate set — it is a **ceiling** on it. Every
deterministic predicate answers *did the world reach this shape*. Whether a test asserts the right
thing, failing for the right reason, is a question about intent, and no exit code answers it.

**Decided: every gate has two tiers.**

| tier | what it answers | who |
|---|---|---|
| deterministic | did the world reach this shape | the existing `needs`/`decide` evaluator |
| adversarial | is this the *right* shape for the task that was asked | independent judges, prompted to refute |

Four properties make a non-deterministic component safe inside a deterministic loop:

1. **The judge is a veto, never a vote.** It runs only after the deterministic tier holds, and it
   can only refuse. It can never advance a step the deterministic tier declined. The worst it can
   be is noisily strict, which costs a retry rather than a bad merge.
2. **The judge never sees the Drone's transcript or its claims.** It gets the task in the words it
   was given in, the diff, and the deterministic facts. The Drone's own account is precisely the
   contamination the gate exists to remove.
3. **The question is narrow and the answer must cite.** Not *"review this work"* — that is
   expensive and unreliable. *"Does this assertion fail because of the behaviour the task
   describes, or for an unrelated reason? Quote the assertion."* Binary, adversarially framed,
   default-refuse when uncertain.
4. **The refusal is the artifact.** *"REFUSED — the test asserts `result.is_err()`, but the task
   describes a wrong value, not an error"* is simultaneously what the Drone needs in order to fix
   it and what the owner needs in order to understand what happened. **One artifact, not two.**

**It is not expensive, which was the objection worth checking.** A reproduce-step judgement is the
task text, the test diff and the failure output — roughly four thousand input tokens and three
hundred out. On Haiku that is under a cent; three independent judges on every gate in a workflow
cost less than one compile.

Property 4 is the one to build the surface around. `helm` grew to 77,906 lines building visibility
*separately* from the gate, and it still cannot tell the owner what a Drone produced. The gate
already has to explain itself to the Drone. Explaining it to a person is the same sentence.

## 3 · Four steps are the owner's, and `human_approves` becomes real

`write-plan`, `write-design`, `explore-codebase` and `review-diff` have no deterministic tier and
cannot be given one — there is nothing an exit code can say about a design document. Offered a
judge that decides them, a judge that only vetoes them, or a rule that they always reach him, the
owner chose the third: **a design or a plan is his to approve, and the judge's job is to prepare
the summary that makes approving cheap.**

**This makes `human_approves` a build item rather than an enum variant.** Three workflows already
depend on it and no surface has ever asked the question.

**It also changes what `bug.yml` ends at, by transitivity.** `review_clean` is settled by a child
Job running `review.yml`, whose only step is `review-diff` — so every bug run now stops at the
owner's desk rather than at an open PR. That is the intent: it is the step where he most wants to
see the work.

## 4 · The console is a browser. There is no TUI

The Bridge, [`003`](003-bridge-command-centre.md), [`033`](033-the-command-centre-designed.md) and
[`035`](035-the-bridge-becomes-a-ratatui-application.md) all assume a terminal surface. The
assumption was stated plainly and was reasonable when it was made:

> *"I was going with a TUI because I figured integrating with Claude Code and running Claude Code
> sessions was going to be the easiest if you're already in the terminal."*

**It does not hold, and the code says so.** `crates/fleet/src/drone.rs` starts a Drone with
`StdioMode::Log` — `claude` with `--output-format stream-json`, piped to
`~/.armada/jobs/<uuid>.stream.jsonl`. **A Drone is a headless process writing JSONL.** Nothing
watches a terminal. Dispatch, monitor, decide and review all read JSON off disk and could render
anywhere.

Two findings settled it, both from drawing the same moment twice — a Job at its human gate, with
the diff it produced, the judge's verdict, and a reply going back.

**The terminal cannot hold the artifact.** It holds a Job's *state* comfortably: steps, gates,
verdicts, spend. What it cannot hold is the thing being asked about — five visible diff lines out
of three files, a reply box with no real editing, a design document at seventy-six columns. The
artifact is the entire reason the owner is being asked.

**Review and decide are one loop, not two moments.** The owner's correction:

> *"The reviewing and deciding is often a back and forth exercise. I can review something, and what
> if it needs tweaked or changed, then I have to provide feedback that gets sent to the drone."*

A split that puts review in one medium and the reply in another cuts a single loop across two
surfaces, which is worse than either.

**Boarding is not the terminal's argument either.** It was first scored as the one thing a browser
could not do, on the assumption a page could only hand back a command to paste. That was wrong.
`armada serve` is a local process holding the same subprocess seam that already spawns Drones — it
can run the terminal emulator itself, or open a pty over a WebSocket. Boarding from the browser is
*better*, because the owner clicks the Job he was reading rather than finding it again by name.

**What the terminal keeps** is the glance: it is on screen already and a browser tab is not. That
is one advantage, and it does not need a TUI — `armada status` printing four lines covers it.

**Decided:**

| | |
|---|---|
| terminal | `armada spawn`, `armada status`, `armada board` — plain commands |
| browser | monitor, review, decide, reply, board. Served by `armada serve` off the JSONL that already exists |

**No multiplexer is a dependency.** `bridge.rs`'s `Workspace::Cmux` is already a probed, optional,
single-argument handoff. It becomes a config key naming an argv, so cmux, tmux, a terminal
emulator or nothing are all the same mechanism.

## 5 · It is a rewrite, in this repository, behind a deletion

The four decisions above retire most of the tree. `helm`'s 77,906 lines go with the TUI, and much
of `core`'s 39,868 exists to serve it. What survives is `manifest`, `guild`, and roughly five per
cent of the rest.

**The expensive thing was never the lines.** It is `docs/traps.md` — 1,586 lines of measured
behaviour, acquired a day at a time: `SIGPIPE` is `SIG_IGN` at startup, `--output-format
stream-json` requires `--verbose` and `--help` does not say so, `setsid` is not in `std` and is
mutually exclusive with `process_group(0)`, `killpg(SIGTERM)` kills nothing if the leader ignores
it, an IPv6-only listener is invisible to an IPv4 bind probe. **That knowledge is already outside
the code**, which is what makes a rewrite cheap rather than reckless.

**And this project has done it once already.** `.claude/agents/harvester.md` is a clean-room
harvester — behaviour and traps in prose, never implementation, so the source's shape cannot
reproduce itself in the rewrite. `ARCHITECTURE.md` records that its harvest landed. The machinery
and the discipline both exist and have been executed successfully.

**Decided: same repository, new crates, and the old ones deleted before the new ones are written.**

| | |
|---|---|
| carried as code | `manifest` (8,090) and `guild` (9,183) — both work, `manifest` is dogfooded against six fixtures and is used daily. Rewriting them is loss with no gain |
| written new | `fleet`, a slim `core`, `serve` and the console, and a five-verb CLI — around 23,000 lines |
| harvested, never ported | `helm` and the rest of `core` — read for behaviour and traps, nothing copied |

**Deletion is what makes it clean-room here.** A new repository was the alternative; it is not
needed. The old crates leave the working tree in their own commit *after* the harvest is written
and *before* any new code is, so an agent cannot read what is no longer there, git keeps every line,
and the 36 reserved designs and their reasoning stay where they are.

**Three conditions, or this happens again.**

1. **The acceptance test is written first and gates everything.** The reason this tree reached
   145,819 lines is that agents write quickly and nothing constrained them. A rewrite under the
   same process is the same pile in three weeks. This condition is the entire difference.
2. **The Armada-specific traps are harvested explicitly.** `traps.md` covers Claude Code and POSIX.
   The ones that will bite live in the reserved designs and in code comments —
   [`024`](024-the-relay-does-not-fire.md)'s `$PPID` rather than `$$`, boot-id-plus-start-time
   liveness proofs, [`011`](011-what-a-drone-may-do-unattended.md)'s permissions posture,
   [`013`](013-secrets-must-resolve-before-the-run-detaches.md)'s secrets resolving above the
   `--detach` branch, and `setup:` being a list because `&&` is not a shell token. Each is a
   three-line conditional with no comment in the new tree unless somebody writes it down first.
   That is precisely the failure mode `harvester.md` was written to prevent.
3. **The eight architecture principles carry unchanged.** Injected seams, the pure core and its
   reducer, no ambient state, typed failures. They are why this code reads well, they were not the
   problem, and they cost nothing to keep.

## What this supersedes

| | |
|---|---|
| [`003`](003-bridge-command-centre.md) | the *ask* stands — one place for everything. The *screen* does not |
| [`033`](033-the-command-centre-designed.md) | superseded. Seven terminal panels, designed carefully, for a medium no longer being built |
| [`035`](035-the-bridge-becomes-a-ratatui-application.md) | superseded. The diagnosis was correct and the remedy is moot |
| [`020`](020-the-tui-decided.md) | its *"Helm opens beside you"* decision — `↵` hands a worktree to cmux — becomes a config key naming an argv. The other eight are about behaviour and stand |
| [`021`](021-the-work-hierarchy.md) | the **listing** stands — four origins, one sort, `NeedsHuman`. Its *screen* half goes with `035` |
| [`016`](016-what-the-gate-cannot-prove.md) | answered. The narrowing it recorded is what §2 above exists to close |

`PLAN.md`'s two-half plan — one `arm inbox` and the ratatui Bridge — is replaced. Half A's
reasoning about origins and states survives into whatever the browser console lists.

## The rule that holds afterwards

586 commits in fourteen days, 2,181 passing tests, and the loop has never run once. **The feedback
signal was *the tests are green* instead of *it works*.** Green tests are cheap to produce and they
reward building outward, because each new thing can be green on arrival without the whole ever
being exercised.

So: **nothing enters the build that does not move the acceptance test** — one hermetic end-to-end
run of `bug` on a real failure from the corpus, asserting on every gate decision and on what the
console showed at each one. It is written before the code it judges, and it is the only definition
of progress this project now has.
