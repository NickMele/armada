# Drone

**What it is:** The execution runtime for a single Job — a Claude Code process with Armada's injected toolset and its own worktree. Submits Evidence; Fleet decides whether it passed.

---

**Kind:** Agent, Entity.

Companion to the main Armada brief, [Job](job.md), and [Workflow](workflow.md).

## Core principle

**Drones cannot be trusted to manage their own state**, even with a limited toolset and clear instructions — confirmed repeatedly in prototype use. A Drone works its Job and submits Evidence saying it is done; **Fleet decides if that's actually true.** Same principle as [Workflow](workflow.md): self-report is a signal, not a source of truth.

## Lifecycle

**1:1 with a Job.** A Drone is spawned fresh when a Job dispatches (post-approval), and terminates when that Job reaches any terminal status (completed_success / completed_failed / rejected / killed / superseded). No reuse across Jobs — a new Job always gets a fresh Drone.

**1:1 at any moment, 1:N over time.** A Job may have successive Drones over its life: redispatch and Debug's Kill & Redispatch both end a Drone *without* the Job reaching a terminal status.

**No independent state machine.** A Drone is a pure shadow of [Job.status](job.md). Fleet drives every transition on both; the Drone never transitions anything itself.

| Job status | Drone | Liveness clock | Worktree |
| --- | --- | --- | --- |
| `running` | Alive | **Runs** | Held |
| `awaiting_review` | Alive — same PID, worktree and session | Suspended | Held |
| `escalated` | Alive | Suspended | Held |
| `interrupted` | Gone. The process died; the Job did not | — | Held, and **never swept** |
| Terminal | Gone | — | Held until past retention, then swept at Fleet start |

A human gate costs no session context, because the PID, the worktree and the session all survive it.

**Removal is driven by Job retention, never by process exit.** Deleting on merge would strand a retained Job, because Evidence references paths. A killed Job's worktree is held because redispatch may reuse it.

**An `interrupted` worktree is never swept.** Why: it may hold uncommitted work, and destroying that on a restart is the same class of act as auto-killing, which Fleet never does. A person decides.

**The liveness clock suspends at a human gate.** Why: a Drone waiting on a person has no activity by construction, so every gate that outlasted the heartbeat timeout escalated its own Job as `stalled`. Design Plan gates on a human every iteration, so the loop shape hit it first and hardest.

While suspended, Fleet stops expecting heartbeats and `poke_limit` does not advance. **What bounds a Job sitting at a gate is Drone/Job timeout and worktree cleanup policy, not the liveness timer.** Fleet still reconciles dead processes against live Jobs on restart, which is how a Drone that dies during a gate is caught.

**A healthy Drone accepts Redirect, Kill and Pause.** All three are available on a non-escalated Drone rather than reserved for escalated ones.

- **Redirect** is mid-step context injection, and it is recorded on the Job. It must never silently become a step restart: the Drone continues with more information, and the record says a person intervened.
- **Kill** is unambiguous on a healthy Drone and already safe. Nothing about the escalated state makes killing safer.
- **Pause** only means anything while a Drone is healthy, since anything escalated is already paused.

## Composition

A Drone is a Claude Code CLI process in headless mode, an isolated git worktree on its own branch, and an injected toolset.

The injected toolset:

- Skills / Agent file (Kit-only, see [Kit](kit.md))
- Sub agents (Kit global + Manifest project-specific)
- MCP (resolved via Kit → Manifest inheritance)
- The project's allowlist
- The [Manifest Commands registry](manifest.md)
- A brokered secrets scope
- A dedicated **Evidence MCP tool**

All of that assumes one owning Manifest. For a Drone working a [Convoy](convoy.md), see the resolution rule below.

Discovery needs nothing from Armada for the MCP half: tools are self-describing, and what the prompt supplies is the obligation a schema cannot state.

**The brokered scope never includes a Git credential.** A Drone commits locally, inside its own worktree, and the Drone-facing `VCS` type has no push method at all. Push, pull request and merge are [Fleet](fleet.md)'s, using credentials Fleet holds directly.

**A [Convoy](convoy.md) Drone's single worktree spans every declared Workspace's directory** — still one Drone, one worktree, one branch. Why: every declared Workspace descends from a single root `armada.yml`, so a Convoy is root-Manifest-scoped and cannot span repos, which is what makes one worktree spanning Workspaces ordinary git.

Where a worktree and its log live on disk is in `../contracts/system-architecture.md` section 7. It is not configurable, and is derived rather than stored.

**A worktree outlives the Drone using it.** On a scope revision Fleet terminates the Drone, re-resolves configuration against the new Manifest set, and spawns a fresh Drone **on the same worktree and branch** — the same path [Pilot](pilot.md)'s Restart Step already uses. Consistent with a Drone being 1:1 with a Job at any moment and 1:N over time.

**A narrowing proceeds unchallenged; a widening returns to the dispatch approval gate first**, so a respawn against a widened scope happens only after a person has approved that scope. What is lost is session context, not work: Facts and Evidence live on the [Job](job.md).

**The resolution below runs again on every respawn.** Since permissions intersect, a widened scope can only produce a narrower toolset than the Drone that asked for it — Commands excepted, since they union.

**What Armada injects is not what the process ends up holding.** Measured against the live CLI: `--allowedTools` is a permission allowlist rather than a toolset — it removed none of the thirty built-in tools, and a spawned Drone inherited the operator's own MCP servers, plugins, subagents, skills and SessionStart hook. **Isolation is opt-out, and the opt-out is not** `--allowedTools`**.**

**For MCP servers, `--strict-mcp-config` is the opt-out, and it is not optional.** A harness must let Fleet inject the Evidence MCP server, since Evidence is the only sanctioned completion path, and must then run with that server and nothing else.

The two fail differently: a harness that cannot inject fails loudly and immediately, while one that injects but cannot exclude looks like success — the Drone works, and holds the operator's whole toolbelt. Both are enforced at compile time rather than by a runtime check, because `mcp_config` and the strict-mode field are non-optional on `DroneSpawnConfig` with no escape-hatch constructor. See `../contracts/adapters.md`.

**Built-in tools are a separate problem and still open** (see Open questions). `--strict-mcp-config` bounds MCP servers, not the thirty tools the CLI ships with, so the resolution table below governs what Armada grants rather than what the Drone can reach — the part the intersection rule exists to bound.

### Convoy resolution — permissions intersect, knowledge unions

A Convoy is one Drone under several Manifests, so every injected item needs a rule for what happens when they disagree.

| Injected item | Resolves | Why |
| --- | --- | --- |
| Allowlist | **Intersection** | Only ops *every* declared Manifest allows |
| Secrets scope | **Intersection** | Only secrets *every* declared Manifest grants |
| MCP | **Intersection** | A callable tool is a permission. Only servers every Manifest grants |
| Sub agents | **Intersection** | Only personas every declared Manifest defines |
| Commands | **Union, namespaced by Manifest `id`** | Namespacing leaves nothing to intersect |
| Ports | **Union, qualified by Manifest `id`** | A port is knowledge rather than authority |
| Skills | **Union** — the exception | A Skill is *instructions*, not a permission |
| Agent file | **Union** | Instructions, not authority |
| Evidence MCP tool | Unaffected | Armada's own, injected identically regardless of Manifest count |

**Secrets.** A Drone unable to reach a secret because another Manifest withholds it is a visible, debuggable failure, not a silent scope violation.

**Commands.** `api:migrate` and `billing:migrate` are two commands, not one name with two meanings, so there is nothing to intersect. A Convoy Drone is legitimately working in every gating Workspace and needs their Commands to do the work; intersection is monotone, so it gave the widest Convoy the smallest toolbox. The namespace protects better than the intersection did — a prefixed name cannot be invoked in the belief that it belongs to another Workspace.

**Ports.** Injecting a port number grants no ability the Drone lacked — it could already bind any port, and the allowlist is blast-radius reduction rather than a sandbox. Colliding `env` names across the Job's Manifest set are rejected at claim time.

**Skills.** A Skill grants the Drone no ability it did not have, so there is no authority to widen and intersection has nothing to protect. Intersecting them would also be near-vacuous — Skills are repo-specific and rarely overlap — leaving a Convoy Drone **less** capable than one working either Workspace alone, which is the opposite of the intent. Contradictions between two Workspaces' Skills are a prompt-quality problem, not a security boundary.

**The rule in one line: a permission intersects, knowledge unions.** What intersection protects is *authority* — whether a Drone working under several Manifests ends up holding power no single gating Manifest granted. Instructions grant none, so unioning them widens nothing.

This is the peer axis, Manifest against Manifest. The Kit → Manifest direction rule is withdrawn and does not bear on it.

**Commands are the one item on the union side that is still a permission.** Namespacing removed the conflict the intersection existed to resolve, not the authority. The rule still holds for everything sharing a flat namespace; MCP servers are the closest case and were not namespaced.

**The resolution rule is what makes a Convoy Drone spawnable.** Without it, Skills, Sub agents, MCP and Commands are undefined for a Convoy, so there is no boot configuration to spawn against.

### What's frozen at spawn vs. live

Two different reasons, not one. Some items are frozen because a process cannot change them mid-session; others are frozen deliberately, because letting them move would let a Drone move its own yardstick.

| Item | Frozen or live | Why |
| --- | --- | --- |
| Skills, MCP, Agent file, Sub agents, Voice/tone | Frozen at spawn | Boot-time constraint |
| Commands | Frozen at spawn | A Drone that could write itself a Command could grant itself one |
| Checks that existed at spawn | Frozen for the life of the Job | A yardstick cannot move while the work is judged against it |
| Checks added mid-Job | **Live** | **Additive-only.** Adding a gate is not weakening one |
| Allowlist, budget caps, dispatch freeze | Live | Re-evaluated at every gated checkpoint |

The boot-time constraint: injected MCP servers cannot be swapped, nor the system prompt rewritten, without a kill and respawn. Commands and Checks are frozen by choice rather than by that constraint.

Fleet resolves Checks from the spawn snapshot rather than live — the rule `acceptance_criteria` already follows. A Job may write a new Manifest or a new named Check and have it gate.

A change to a frozen item during a running Job takes effect on that Job's *next* Drone (redispatch or a future Job), not the current one.

**A gate-definition change is detected and routed to a mandatory Judge look**, reusing the pattern used for declared-plan drift. Freezing prevents the narrowing; it does not make the attempt visible, and a Drone that tries is worth knowing about.

**A rescope-and-respawn re-snapshots.** So narrowing a Check and then triggering a scope revision would freeze the narrowed version in. That is caught at the scope-revision approval gate rather than here.

## What a Drone is told

The toolset above is what a Drone can **do**. What it is **told** — the framing wrapped around the task before the task itself — is governed by `../contracts/agent-prompt.md`, sibling to the Copy Contract.

What it settles that bears directly here:

- **Six layers, assembled in order.** Baseline → Kit → Manifest → WorkflowDef → task → step. **Freeze time orders them** — the boot-time constraint in the table above is the derivation, not a preference. A block that cannot be rewritten mid-session must precede the block rewritten at every step boundary.
- **Layer 1 is not configurable.** A Kit or Manifest able to edit the baseline could delete the sentence making Evidence the only completion path.
- **A running Drone's system prompt never mutates — but Fleet injects turns into a live session.** Each turn is a row in the prompt library under `Kind = Injected turn`, carrying its trigger and its wording. The mechanism is settled; a row with an empty Wording column is a turn with no sanctioned copy.

**The baseline's wording and how a Drone discovers its Commands and MCP tools are one surface seen from opposite ends**, and are settled together.

The Agent Prompt Contract exists because every constraint on Armada's prompts was written down and none of the prompts were. Section 5 lists the clauses the baseline must carry — completion is claimed through the Evidence tool and nowhere else, stopping and handing back through `escape_hatch` is a legitimate way to finish, a denied command is denied rather than an obstacle to route around, and secrets are brokered and never held.

**The Drone is never told what the Checks are.** "The suite is run against your work" says a gate exists without naming the bar. Naming it hands the Drone a target to satisfy rather than work to do, and `check_config_edited` is already a gaming pattern.

The same rule keeps every counter out of an injected turn — a counter is a bar, and a Drone one attempt from escalation has the strongest possible incentive to satisfy it rather than do the work.

### Worked samples

The samples below, the assembled wording, and the decisions visible in each are on the **Drone** row in the prompt library, with the rest of the assembled prompts.


#### Bug, part 2 of 4

```
┌─ BASELINE ──────────────────────────────────────
│ You are working in a git worktree on a branch of your
│ own. You cannot push, open a pull request, or run
│ commands this repository has not declared.
│
│ When you have finished the work described below, you
│ must report it using the evidence submission tool you
│ have been given. It is the only way to report. Work
│ you do not submit is work no one sees, and the task
│ will not move on.
│
│ Submitting returns "recorded". That is a receipt, not
│ a verdict — your work is checked after you submit. If
│ it does not pass you will be told in a later turn,
│ with the reason. Wait for that turn.
└──────────────────────────────────────────────
┌─ JOB BRIEF ───────────────────────────────────
│ Repository: armada
│
│ Dispatching two Jobs against the same repo leaves the
│ second one's worktree at a path the first already
│ registered, and it fails with a message naming neither.
└──────────────────────────────────────────────
┌─ WHERE YOU ARE ────────────────────────────────
│ This task runs in four parts. You are on part 2.
│
│   1. Plan the change      ✓ done
│   2. Implement            ← you are here
│   ─────────────────────────────────────────────
│   ▌ STOP. Submit when part 2 is done, then wait.
│   ─────────────────────────────────────────────
│   3. Run the suite        ✗ not yours — do not run it
│   4. Summarise            ✗ not yours — do not write it
│
│ What part 1 produced:
│   "The worktree path is derived from the repo name
│    alone, so a second Job collides. It should carry
│    the job id. worktree.rs, add() — the path is built
│    at line 40."
│
│ Parts 3 and 4 happen after you submit, and doing them
│ yourself does not move this task forward. Leave the
│ branch in a state they can start from.
└──────────────────────────────────────────────
┌─ STEP: Implement ────────────────────────────────
│ Make the smallest change that addresses the cause
│ identified in part 1. Do not fix adjacent problems you
│ notice — say so under Not claimed instead.
│
│ Read before you write. The rust-conventions skill
│ covers error handling and module layout here.
│
│ What you claim should be what the work now does, not
│ that you finished.
└──────────────────────────────────────────────
```

#### Code Review, part 2 of 3 — the inverted case

The test of the layering, since the diff is the Drone's **input** rather than its output.

```
┌─ BASELINE ──────────────────────────────────────
│ [identical to the sample above, on every step of
│  every Job. Mechanics, never task content.]
└──────────────────────────────────────────────
┌─ JOB BRIEF ───────────────────────────────────
│ Repository: armada
│
│ Review PR #218, which reworks how worktree paths are
│ built. It touches the dispatch path, so pay attention
│ to what happens when two Jobs start at once.
└──────────────────────────────────────────────
┌─ WHERE YOU ARE ────────────────────────────────
│ This task runs in three parts. You are on part 2.
│
│   1. Read the changes     ✓ done
│   2. Assess               ← you are here
│   ─────────────────────────────────────────────
│   ▌ STOP. Submit when part 2 is done, then wait.
│   ─────────────────────────────────────────────
│   3. Deliver              ✗ not yours — do not post it
│
│ What part 1 produced:
│   "6 files, 340 lines. worktree.rs add() now takes a
│    job id and builds the path from it. dispatch.rs
│    passes it through. Three call sites updated, two
│    tests changed, one added."
│
│ Part 3 posts your review. Doing it yourself does not
│ move this task forward.
└──────────────────────────────────────────────
┌─ STEP: Assess ───────────────────────────────────
│ The changes are below in full. You did not write them
│ and you are not fixing them — you are reviewing them.
│
│ Write your findings to REVIEW.md. Tie every finding to
│ a specific file and line. A finding that would apply
│ to any diff is not a finding.
│
│ "No issues" is a legitimate conclusion on a small,
│ clean change. It is not a legitimate conclusion you
│ reach quickly on a large one.
│
│ Do not edit the code you are reviewing.
└──────────────────────────────────────────────
┌─ THE CHANGES ──────────────────────────────────
│ diff --git a/crates/vcs/src/worktree.rs ...
│ [340 lines, injected]
└──────────────────────────────────────────────
```

**What the inverted case forced.** A fifth block — the diff is injected as its own section *after* the step instructions, because it is reference material rather than an instruction. And `Do not edit the code you are reviewing`, which stops a review Job quietly becoming a fix Job: a coding Drone's instinct on seeing a diff is to improve it.

Everything above the step block is structurally identical between the two samples. Only the step text and the injected material differ.

## Evidence reporting

Structured only — through the Evidence MCP tool, not plain text and not an ambiguous "I'm done." This is the one sanctioned way a Drone submits Evidence that its Job is done, and it substantially closes the "claims done in plain text, bypasses structured Evidence" failure mode from [Workflow](workflow.md).

It does not rest on output parsing, which was the point of making it a tool call. **A headless run denied every tool it needed still terminated reporting success, with** `is_error` **false and exit code 0**, having accomplished nothing. The envelope agrees with itself and is wrong, so what Fleet accepts as proof cannot be the exit code.

**Fleet reads the run's result for cost and turn count, and for nothing else.** Exit code, `subtype`, `is_error` and `stop_reason` gate nothing. All four were present, agreeing, and wrong, so a gate written against any of them passes a Drone that did nothing. **No cheap signal goes beside the tool call.** Evidence submitted through the tool stays the only proof.

**What is read instead explains an empty result rather than judging one.** `permission_denials[]` carries every refused call with its full input, and `tool_result_meta[].non_execution_kind` separates a call that was refused from one that ran and failed.

A run ending with no evidence and at least one refusal is `blocked_by_policy` — a different condition from `silent`, which is a Drone that called nothing at all. Both come back empty and look identical in the envelope, and only one of them can be fixed by rewording the task.

### Fleet's response to Evidence

| Fleet receives | Fleet does | Counts against retry_limit? |
| --- | --- | --- |
| Valid evidence, passes Mechanical + Judge check | Advances the workflow step | N/A |
| Valid evidence, fails Mechanical or Judge check | Standard gate-failure retry flow | Yes |
| Missing or insufficient evidence | Prompts the Drone for more — a free clarification round | No |
| No evidence, and at least one tool call refused | Escalates as `blocked_by_policy` | No |

- **On a gate failure**, the refusal reprompt carries the Judge record's `expected` and `produced` back to the Drone — never `consequence`, and never a counter.
- **On missing or insufficient evidence**, the clarification round is capped, then escalates as `stalled`. `silent` is a sub-kind of `stalled`, not a separate trigger. The cap's value is a config row and is still undecided — tracked in `../contracts/configuration.md`.
- **On `blocked_by_policy`**, the refused calls and their inputs go on the payload. No clarification round is spent first — asking again cannot produce a tool the Drone is not permitted to call, and nothing was attempted that a Check could fail.

**Two distinct counters, not one.** This clarification-round cap — content arrived via the Evidence MCP tool but was not sufficient — is a separate counter from `poke_limit`, the liveness nudge used where nothing structured arrived at all. Both can end in the same `stalled` label, which reads ambiguous, but they check different things and are tracked independently. See [Workflow](workflow.md).

**The cap has no field name and no counting scope**, unlike its sibling `poke_limit`. Per Job, per workflow step or per loop iteration is undefined, and on a loop workflow a per-Job cap exhausts inside the first iteration. Separate from the value question (see Open questions).

## Runtime data (not state)

Tracked per-Drone while alive, feeds Monitor Active Work's heartbeat and Respond to a Push Alert's trigger-adaptive Debug views:

PID, worktree path, current workflow step, elapsed time, turn count, resource usage, heartbeat timestamp.

**The turns themselves are [Observe](observe.md)'s.** The heartbeat says a Drone is moving; watching what it is actually doing is a read off the transcript Fleet is already parsing, and it takes nothing over.

## Compliance & policing

Fleet monitors a Drone for a known set of failure modes — Silence/Stalled, Claims-done-no-evidence, Claims-done-plain-text-bypass, Thrashing, Evidence gaming. Full detail lives on [Workflow](workflow.md), not duplicated here.

## Sub-dispatch

A **sub-dispatch** is a Drone spawning a **Job** — its own workflow, its own worktree, its own rail, its own Job Board row — needed to complete its assigned Job. Auto-approved, no separate human gate, unless a fan-out abuse threshold trips (see [Workflow](workflow.md) — Dispatch Approval, Two Levels).

**A Judge call is not one**, so the fan-out cap counts Jobs only — [Judge](judge.md) owns why, and [Job](job.md) records `spawned_jobs` and `judge_calls` separately.

Not to be confused with the **Sub agents** configuration concept (Kit/Manifest — defined agent personas available to a Drone). Sub-dispatch is the runtime *act* of a Drone spawning a sub-Job; Sub agents is the *config* defining what's available to invoke.

## Escape hatch

Every Drone carries one further injected tool, `escape_hatch`, which ends autonomous execution and hands the Job to a person. Full design lives on [Pilot](pilot.md). Reading what a Drone is doing without ending it is [Observe](observe.md).

- **Drone-initiated.** The Drone calls it on its own as a stuck signal, in place of thrashing or claiming a completion it did not reach.
- **Human-initiated.** Fleet marks the Job, then instructs the Drone to call it once the engineer confirms Pilot in Bridge.

**A pull succeeds only on a Job Fleet has marked for the handoff.** An unmarked pull is refused, and the Job escalates as `hatch_unbidden` rather than passing to `piloted`. Why: the stuck signal still reaches a person, by escalation rather than by clean exit.

**A Drone-initiated escape hatch does not count against the repeat counter.** Why: counting a sanctioned exit as a repeat failure penalises the behaviour the mechanism exists to encourage, and the counter drives the escalation payload's suggested action, so a Drone that correctly raised its hand would still be described to the engineer as a repeat failure.

Accepted cost: a step that two successive Drones both escape-hatch out of will not show as a repeat, so that pattern has to be read from the escalation history instead.

The Drone's only contribution is a narrative of what it is stuck on, passed as the tool argument in three named fields — `trying_to`, `blocked_by`, `tried`. It is not Evidence and does not go in the Evidence table: Evidence is proof tied to an advance gate, and this says no proof is coming.

It lands in the handoff bundle, and Fleet assembles everything else in the handoff. The session that opens for the engineer runs at a Kit-level unrestricted toolset, which is the point: the narrow toolset is the thing being escaped.

## Written output

A Drone writes text that leaves Armada permanently: commit messages and PR descriptions land in a real repo and are read by humans who are not you. That text is governed by `../contracts/agent-copy.md`, which sits under `../contracts/design-system.md`. Text going the other way — what a Drone is **told** — is governed by its sibling, `../contracts/agent-prompt.md`.

The surfaces a Drone writes to are rows in the Copy registry under `Written by = Drone`, each carrying its enforcement, its reader and its samples. Enforcement splits by destination rather than by surface: text landing in a real repo is read by people who did not ask for it, which is what earns a hard gate.

**A gate failure gets one free correction round that does not consume the retry budget.** Why: without the free round a style bounce spends `retry_count`, and enough of them escalate the Job as `gate_failure`. It reuses the one-free-round mechanism for present-but-insufficient evidence above, so a phrasing problem can cost a turn and can never escalate a Job.

**Seeded with real samples, not more rules.** The Drone prompt carries curated exemplars from actual pre-AI commit history. Corpus build is an open task on the Agent Copy Contract, targeted at the M0 v1 harvest.

**Open collision:** the Manifest-level `Commit/PR message template` setting could mandate a format the lint rejects — tracked in `../contracts/configuration.md`.

## Open questions

- **[drone-builtin-tools-confinement]** How is a Drone's toolset actually confined, given that `--allowedTools` removes none of the built-in tools? `--strict-mcp-config` bounds MCP servers only, not the thirty built-in tools the CLI ships with.
- **[drone-evidence-clarification-cap-scope]** What is the evidence clarification-round cap's field name, and what does it count against — per Job, per workflow step, or per loop iteration? Unlike its sibling `poke_limit`, it currently has neither a name nor a counting scope, and on a loop workflow a per-Job cap would exhaust inside the first iteration.
