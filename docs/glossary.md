# Armada — glossary

The vocabulary, in one place. Every term below is used in exactly this sense in CLI output, MCP
tool names, code, and every other document. **If a word here starts meaning two things, that is
a defect, not a style preference** — the last time it happened, `FAIL` and `FAILED` coexisted
for a release and the fix cost more than the rule ([`PLAN.md`](PLAN.md) §3).

This page is derived from the decisions recorded in [`ARCHITECTURE.md`](ARCHITECTURE.md) §1.9
and [`PLAN.md`](PLAN.md) §13–§15. Where it disagrees with those, they win.

## The four modules

They stack, and nothing points upward ([`ARCHITECTURE.md`](ARCHITECTURE.md) §1.9).

| Term | Is | Is **not** |
|---|---|---|
| **Armada** | The whole suite — one binary, four modules, plus the skills, hooks and MCP server it ships. | A daemon. Nothing runs in the background between commands. |
| **Manifest** | The workspace module: what a repository is, how it is configured, owned and reclaimed. Its file is `armada.yml`. | Merely "stack config". The ownership store is the part nothing else has — it is what lets a workspace be reclaimed after its directory is gone. |
| **Guild** | You — voice, skills, hooks, subagents, workflows. Machine-global in `~/.armada/guild/`, synced between your machines. | Repository content. No part of it is ever committed to a project. |
| **Fleet** | **Every Job on this machine, and the Drones executing them.** A collection. `ls`, `reap` and `inbox` are operations on that set; `spawn` adds to it. | A manager. Nothing in Fleet *drives* a Job — a Job drives itself (see **Job**). Nor a scheduler you talk to: you talk to Helm. |
| **Helm** | The one agent you talk to. A Claude Code session running an orchestrator persona with Armada's MCP server as its toolbelt. | A binary. See *Why there is no `helm` on `PATH`* below. |

## The work

| Term | Is | Is **not** |
|---|---|---|
| **Signal** | Something raised that may or may not be acted on: a **task** you intend, a **report** you filed, a **failure** Armada noticed, an **untried** verb, a question a Drone **asked**. One store, one id space, one way to promote any of them into a Job. | A queue that drains on its own. A signal is raised and then ignored, dismissed, or promoted — and only the last of those makes work. |
| **Job** | The durable unit of work: a uuid, a git worktree, a port block, a transcript, a budget, and eventually a verdict. Survives crashes, reboots, and the death of its Drone. **It is also the state machine**: it holds its workflow, the step it rests on and what it has spent, and `armada fleet tick` is one transition of it. | A process. A Job with nothing running is the ordinary resting state. Nor a thing something else drives — there is no manager above it, which is why no such word exists here. |
| **Drone** | The process executing a Job. Temporary by design: it runs one exchange and exits. | The work itself. Killing a Drone does not end its Job, and a Job outlives every Drone it ever spawns. |
| **Contract** | **The guarantee a verb makes** — what `armada manifest check` promises to do and what states it may end in. Used this way throughout [`PLAN.md`](PLAN.md) §3 and the `--json` envelope. | A task. That is a Job. |
| **Skill** | Repo-local knowledge: a named grant plus a pointer to prose ([`PLAN.md`](PLAN.md) §4.8). The mechanical half lives in `armada.yml`; the prose is a markdown file Manifest never parses. A guild skill is the same idea, owned by you rather than the repo. | A script. It has no `cmd:` and cannot be run — only listed, resolved and rendered. |
| **Workflow** | An ordered set of steps in your guild — `design`, `plan`, `feature`, `bug` — naming which skill runs each step and what verdict advances it ([`PLAN.md`](PLAN.md) §14.4). | Hardcoded. It is data, editable at one in the morning. |
| **Projection** | Copying the guild's mechanical half into the directories Claude Code reads, tracked by a manifest of what was placed and a hash of each file ([`PLAN.md`](PLAN.md) §13.2). A guild is on no tool's load path until something projects it. | A sync. It runs one way, and a file you edited is left as it is and reported rather than overwritten. |

**One Job has at most one live Drone.** The two words exist because their lifetimes differ, not
to name the same thing twice ([`PLAN.md`](PLAN.md) §14.1).

## The surfaces

| Term | Is | Reached by |
|---|---|---|
| **Helm** | The conversation. Decompose, delegate, aggregate, report. | `armada`, or `armada helm` |
| **Bridge** | The live screen. Every Job and its state, redrawn in place like `htop` or `k9s`. | `armada bridge`, or `/bridge` from inside Helm |
| **Board** | Taking a Job over yourself — you get its worktree and a `claude --resume`. | `armada fleet board <job>` |

Helm is where you **talk**; the Bridge is what you **watch**; boarding is how you **take the
wheel**. None of the three owns the others, and Helm works with neither of the other two built.

## Status vocabulary

Three enums, each owned by one module. **One spelling everywhere, and it is the JSON spelling**
— SCREAMING in both the payload and the human render, `FAILED` and never `FAIL`.
`crates/core/src/error.rs` enforces this with a test for the first of them; the other two
inherit the rule.

| Enum | Owner | Values |
|---|---|---|
| **Status** | Manifest | `READY` `UP` `DOWN` `CLEAN` `PASS` `OK` `SKIPPED` `PARTIAL` `FAILED` `ABORTED` `DEAD` `TIMEOUT` `RUNNING` `WAITING` |
| **Job state** | Fleet | `QUEUED` `RUNNING` `PAUSED` `STALLED` `BLOCKED` `ABORTED` `DONE` |
| **Verdict** | Fleet | `PASS` `FAILED` `BLOCKED` `NEEDS_HUMAN` |

**Why Manifest's Status is not simply extended to cover Jobs.** `BLOCKED` is a legal Job state
and a legal verdict, but it is deliberately not a Manifest terminal state: exit codes are
`f(error.class)`, a blocked run carries no class, so a merge gate would read exit `0` as
success ([`PLAN.md`](PLAN.md) §3.1). The constraint applies to one enum and not the others,
which is exactly why they are separate.

## Words deliberately not used

Recorded rather than deleted, because a rejected word with no reason attached gets proposed
again in six months.

| Word | Proposed as | Why not |
|---|---|---|
| **Berth** | An idle pool of pre-warmed Drones awaiting assignment. | There is no pool. Jobs are minted per task, and a pool of live idle processes is a daemon under another name ([`PLAN.md`](PLAN.md) §4.3). The state it was reaching for — work that exists with nothing running — is just a Job without a Drone, which needs no new word. |
| **Contract**, as a task | The unit of work assigned to a Drone. | The word was already load-bearing in its ordinary sense across the `--json` envelope, the verb tables and a dozen doc comments. **Job** took the role instead, and nothing had to be renamed. |
| **Session** | The unit of work. | Ambiguous once Job and Drone are distinguished: it named both the durable record and the running process, which is the conflation §14.1 exists to undo. Still correct for *a Claude Code session*, which is what a Job's conversation actually is. |
| `helm`, as a binary | The orchestrator's command. | See below. |
| **Coverage**, as a verb name | `armada coverage` — which of Armada's own verbs this machine has never run. | Already load-bearing: the CI job is named `coverage` and `AGENTS.md` gates the merge on its ratchet. A second meaning under one word is the exact defect this page exists to prevent — shipped as `armada untried` instead, and the file it counts into is `~/.armada/untried.jsonl`. |
| **Pilot** | The thing that takes a Job and drives its Drone — a name for what `armada fleet tick` does. | The metaphor is exact, which is why it was tempting: a harbour pilot boards a ship she does not own, guides it through the part needing local knowledge, and steps off carrying no cargo. It was rejected because **a Pilot would hold no state**. Everything it would "own" — the workflow, the step, the budget, the verdict — the Job already holds, so the word would name a function rather than a thing, and the Job would still be what knows where it is. The question it was reaching for — *"Drones are ephemeral, so what drives the loop?"* — is answered by the Job driving itself. |
| **Run** or **Voyage**, as the thing carrying a Job through its workflow | A first-class object between Job and Drone. | Same objection as **Pilot**, plus `run` is already taken: Manifest calls a `check` execution a run, with a run directory and a run id. |

## Why there is no `helm` on `PATH`

Kubernetes' Helm owns that name, and Armada is expected to run on machines that have it — the
`python-ml` fixture shells out to `kind` and `kubectl` ([`PLAN.md`](PLAN.md) §6.1). A second
`helm` would shadow a tool the user depends on to do their actual work.

Helm is therefore a subcommand and the bare-`armada` default, never an installed program. The
concept keeps its name; the namespace stays clean.

## Palette

The Bridge and every other coloured surface use one palette, defined once in
[`commands/helm/bridge.md`](commands/helm/bridge.md). **Armada targets truecolor and degrades gracefully; it does
not promise the palette survives at 16 colours** — signal amber and flare orange are one ANSI
step apart and both collapse to bright yellow, taking the `RUNNING` / `STALLED` distinction with
them. Terminals that matter here (Ghostty, cmux, every modern emulator) are truecolor.

## See also

[`commands/reference.md`](commands/reference.md) · [`ARCHITECTURE.md`](ARCHITECTURE.md) §1.9 ·
[`PLAN.md`](PLAN.md) §13–§15
