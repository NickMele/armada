# Agent Prompt Contract

**Kind:** contract. **Governs:** text injected into a model — every prompt
Armada assembles and puts in front of one. Each is catalogued here, one
entry apiece: the invocations in section 2, the injected turns in section
4a. Sibling to the
[Agent Copy Contract](agent-copy.md), which governs text coming out. Both
sit under the [Design System — UI & Voice](design-system.md) contract. Read
before adding or changing any prompt Armada assembles and sends to a
model.

**Purpose:** governs text going **into** a model — every prompt Armada
assembles and injects. Sibling to the [Agent Copy Contract](agent-copy.md),
which governs text coming **out**. Both sit under
[Design System — UI & Voice](design-system.md) and may not contradict it.

**Why it exists.** Every constraint on Armada's prompts is written down
somewhere. None of the prompts are. The Judge is specified to be blind to
the Drone's transcript, `judge_check.prompt_key` is specified as a
reference into a prompt library, Voice is specified as configurable and
injected — and nothing states what any model is actually told. This is
that page.

**Revised Aug 2026** after adversarial review. Sections 1, 2, 3, 5 and 6
were rewritten; section 4a and section 9 are new.

---

# 1. Scope

**Every prompt Armada assembles and puts in front of a model.** That is
the membership test, and it is deliberately different from the one on
[System Architecture](system-architecture.md) section 9, which asks
whether an invocation carries a toolset. That is a taxonomy question with a
different answer: a clarification round is not a separate Agent, but it is
unambiguously a separate assembled prompt.

Every entry catalogued here passes this test — the invocations in section
2, plus the injected turns in section 4a.

**Not in scope:** what a model writes back — [Agent Copy
Contract](agent-copy.md). The task text itself, which comes from the Job.
Model selection and budget, which are per-step configuration.

---

# 2. The invocations

| Prompt | Assembled by | Fires when | Wording |
| --- | --- | --- | --- |
| **Drone** | `fleet` | At spawn, which is once per workflow step | Specified — section 5 |
| **Helm** | `fleet` | Per session | Not specified |
| **Judge** | `verification` | Per criterion, after a mechanical check passed | Not specified |
| **Job-shape classifier** | `fleet` | At Job creation | Not specified |
| **Manifest scanner** | `config` | During the setup wizard's Proposal phase | Not specified |

What each is told and never told is below. Three of the things
[System Architecture](system-architecture.md) section 9 lists are not
entries here:

| Section 9 lists | Here |
| --- | --- |
| Clarification round | **An injected turn** — section 4a. A separate assembled prompt, not a separate invocation |
| Sub-agents inside a Drone | **Not assembled by Armada.** Claude Code assembles it from Kit and Manifest Sub agent definitions, which are layer 2 and layer 3 content this contract governs. Armada supplies the definition, not the prompt |
| Denial-pattern suggestion | **Possibly not a model call.** If it is, it is an entry above. Unresolved on section 9 and inherited here |

**Voice generation is not among them**, for the reason below. An entry
saying so double-counts the thing it exists to exclude.

**Two carry a toolset.** Drone and Helm are Agents; the Judge and the
classifier are model calls. The scanner is the open case. See
[System Architecture](system-architecture.md) section 9.

## What each invocation is told

### Drone

**Told:** the three baseline clauses, the resolved toolset with its
documentation, Voice, **Way I work**, **Expectations**, the workflow step
and its criteria, the Evidence Scope policy, `acceptance_criteria[]`.

**Never told:** secrets, other Jobs' context or evidence, anything from a
Manifest outside its own Job's set, the operator's own MCP servers.

Six layers, assembled in order — section 3. Its wording is section 5.

### Helm

**Told:** a strict subset of the Fleet MCP — all 14 queries, one of 12
commands. The selected Manifest. Its resolved authority, a Machine setting
between rung 1 and read-only with no Manifest tier. The obligation to call
`get_events_since(cursor)` at the start of each turn. Voice.

**Never told:** anything outside the selected Manifest. Secrets.

One of two invocations carrying a toolset.

### Judge

**Told:** the original task text. The resolved Evidence Scope object
carrying the pre-loaded patch — the diff is delivered, not fetched. The
Evidence bundle under judgment. `baseline_ref`'s Evidence where
`gaming_check` is enabled. The deterministic facts. `reference_docs` as
the yardstick, kept separate from `context_paths` as the target. One
criterion's question.

**Never told:** the Drone transcript or self-report, per constitutional
rule 2. The other panel members' verdicts under `panel_size` > 1, per
rule 5.

`judge_check.prompt_key` references a prompt library that is sited
nowhere — section 8.

### Job-shape classifier

**Told:** the request text, the Manifest set, the three shapes.

**Never told:** repo contents. It has no toolset.

A model call rather than an Agent. Why the classifier chose as it did is
recorded nowhere.

### Manifest scanner

**Told:** repo facts gathered by the wizard, or a read-only toolset —
undecided. Regardless of that answer it is told the target repo, the
`armada.yml` schema it is proposing against, and that the Proposal phase
is iterative rather than one-shot.

The open case on whether it carries a toolset, tracked as
`[manifest-scanner-agent-or-model-call]` in
[System Architecture](system-architecture.md).

## Three shapes that differ from the flat Judge row

| Case | How it differs |
| --- | --- |
| **The thrashing pass** | Workflow asks a **three-way** — converging, justified drift, or thrashing. Constitutional rule 3 says yes/no, never "review this." That tension sits unresolved in the source pages and is recorded here rather than papered over |
| **Code Review** | The diff is the Drone's **input** rather than its output. "The work product" needs restating for that workflow, or target and yardstick invert on the one workflow designed to expose the difference |
| **Criterion independence** | Rule 5 governs panel members. **Whether a Judge grading criterion 2 may see criterion 1's verdict is unstated on every page.** Weaker case than panel independence — the criteria are different questions — but a contract about what a model is told should answer it |

## Voice generation is not an invocation

**Corrected Aug 2026.** There is no dedicated Voice model and no Voice
call. Voice is a **layer that rides inside the others** — a Drone writing a
commit message, the Judge writing a refusal summary, Helm writing a reply.
All three already appear above, and Voice already appears as input to two
of them, so a peer row double-counted it.

Kit names three surfaces Voice governs: Judge summaries, Helm replies, Job
summaries. The first two are produced inside calls listed above. **The
third is unaccounted for, and it is the one open question here: what is a
Job summary and who writes it?**

If a model writes it, two parent-contract guarantees break. P6 says
Fleet's own strings should be identical every time, because uniformity is
scannability. And verbs are generated from the status enum by a codegen'd
map with a test asserting every variant has an entry — a test that
guarantees nothing if a model is doing the rendering. If a model does not
write it, Voice governs templating and there is no invocation at all.

**Two things share the word "Voice" and must not be conflated.**

|  | Machine Voice | The Design System voice contract |
| --- | --- | --- |
| Governs | Runtime-*generated* text — job summaries, status updates, what a Drone or Fleet writes about a Job | Static UI chrome — button labels, headings, error strings |
| Configurable | Yes, as a **Machine** setting. One value, no Manifest counterpart, no merge | No |
| Layer here | 2 | Not a prompt layer at all |

Conflating them would make button labels user-tunable, which is not the
intent.

**Recorded disagreement, Aug 22 2026.** Voice classifies as **Machine** in
the settings table, but it sits at **layer 2 beside Skills, the Agent file
and Sub agent definitions**, all of which are **Kit**. Layer 2 is
therefore not one concept's layer, which the layer table below still
implies. Both readings are recorded; neither is picked here.

**Voice is frozen for the Job** for a Drone, alongside Skills, MCP, Agent
files, Sub agents and Commands — changing the setting mid-Job changes
nothing for any Drone that Job spawns, not merely for the one running when
the setting changed.

**Settled: Voice cannot widen or narrow the agent-copy lint, in either
direction.** The lint is fixed. They govern different things — Voice sets
tone, meaning length and formality of runtime prose, while the lint blocks
generated-text tells: puffery, weasel attribution, faux-insight setups.
The case for making it configurable assumed the blocklist holds phrasing a
warm Voice would want, and it does not.

Two things reinforce it. The lint now **hard-gates commit messages and PR
descriptions**, so a widening Voice would let a tone setting weaken a
gate. And the may-narrow-never-widen precedent is withdrawn — it came from
the config direction rule, itself withdrawn once Kit was found to be
defaults rather than a ceiling. Under the Kit/Machine split Voice is a
Machine setting: one tier, nothing to narrow against.

This no longer blocks Phase 4 step 6.

---

# 3. Layers

**Revised Aug 2026** after adversarial review. A Drone's prompt is
assembled from six sources, and **freeze time orders them.**

That ordering was derived from a mechanical constraint: injected MCP
servers cannot be swapped and the system prompt cannot be rewritten
mid-session without a kill and respawn, so a block that could not be
rewritten mid-session had to precede the block that was rewritten every
step. **The derivation is withdrawn and the ordering is not.** A Drone
belongs to a workflow step and every step is a kill and respawn, so all
six layers are assembled fresh at every boundary and none of them is
constrained by another's freeze time.

What holds the order up instead is that it is the one a reader can
predict: earliest-frozen first, so reading down a prompt reads outward
from what nothing can change to what changes every step. That is a
convention, said plainly as one. **Do not reorder the layers** — the
ordering was right for a reason that stopped applying, which is not the
same as being wrong.

| # | Layer | Source | Freezes at | Contains |
| --- | --- | --- | --- | --- |
| 1 | **Baseline** | Armada, compiled in | Build | Section 5. Identical for every Drone, every Job, every Manifest |
| 2 | **Kit** | The Kit file | Spawn | Voice and tone, **Way I work**, **Expectations**, machine-wide Skills, the Agent file, Sub agent definitions. **Voice classifies as Machine rather than Kit** — see the recorded disagreement above |
| 3 | **Manifest** | `armada.yml` | Spawn | Project conventions, repo-specific Skills, project Sub agents, the exemplar corpus (resolved here, injected at layer 6 — see section 6) |
| 4 | **WorkflowDef** | The Job's type | Spawn | What this kind of work means and what finishing it looks like |
| 5 | **Task** | The Job | **Job creation** — earlier than spawn | The request, plus `acceptance_criteria[]` with each entry's source (`ticket` / `helm_drafted` / `human`) |
| 6 | **Step** | Current workflow step | **Never** — re-assembled at each step boundary, like every layer above it | The step's criteria, the Evidence Scope **policy**, the exemplar corpus on steps that write prose |

**Task moved from last to fifth, Aug 2026.** It freezes at Job creation,
earlier than everything above it, and sat last. Under one ordering
principle it belongs ahead of the layer that changes most often. One
consequence survives: the block a reader can rely on is contiguous and
comes first.

The second consequence stated then does not survive. Layer 6 was "the only
thing re-emitted at a step boundary — which is the cheapest place for
re-assembly to sit", and **all six re-emit now**, so re-assembly is not
sitting anywhere cheap. What that costs is a prompt assembled per step
rather than per Job, which is arithmetic on a string and is not the
expensive part of a spawn.

The alternative was to declare prompt order to be recency rather than
freeze time. Rejected: a second organising principle means nothing
decides conflicts between them, and any order becomes justifiable after
the fact.

**The Agent file is Kit-only** — there is no Manifest-level counterpart.
Skills, MCP and Plugins are genuinely two-tier and inherit Kit → Manifest.

## Two rules, not one

**Corrected Aug 2026.** An earlier draft said "later layers narrow earlier
ones and never widen them," borrowing the config direction rule. That rule
was **withdrawn Aug 22 2026** — but this correction was right before the
withdrawal and is right after it, for its own reason: almost nothing in
this stack is a permission, and Drone states the scope in one line,
*instructions grant none.* The corpus at layer 3 narrows nothing — it
adds. Voice at layer 2 tunes length and formality. `acceptance_criteria[]`
supplies content that layers 4 and 6 point at.

| Kind | Rule |
| --- | --- |
| **Instructions** | A later layer may **add** and may **specialise**. It may not contradict an earlier layer. **Layer 1 is unreachable from config** |
| **Permissions** | Not governed here. Allowlist, MCP, Commands, secrets scope and Sub agents resolve per Drone's Convoy table — a permission intersects, knowledge unions, with Commands the amended exception since namespacing removed the conflict intersection existed to resolve |

**Layer 1 is unreachable.** A Kit or Manifest able to edit the baseline
could delete the sentence making Evidence the only completion path, at
which point the completion path is user-configurable and this contract
means nothing. **This one stands on its own footing**, not on the
withdrawn config direction rule the first cut cited: it is a claim about
what config may reach, and layer 1 is reachable from nothing.

**No layer budget exists.** Six layers plus a 60-sample corpus, and
nothing bounds the total. `max_context_size` bounds the Judge's context;
nothing bounds the Drone's. Unresolved, and it is the constraint section 5
is measured against.

---

# 4. Frozen versus live

The Drone spawn rule already distinguishes these. Stated here in prompt
terms.

**Frozen means frozen for the Job, and the snapshot is taken at Job
creation.** Every step of a Job spawns its own Drone, so "frozen at spawn"
would mean re-resolved at every boundary — which would let a Drone weaken
its own yardstick between steps. Fleet snapshots what a Drone works under
when the Job is created and hands the same snapshot to every step's Drone.
See `../concepts/drone.md`.

| Layer | State |
| --- | --- |
| 1 Baseline | **Frozen at build.** Compiled in, unreachable from config |
| 2 Kit, 3 Manifest, 4 WorkflowDef | **Frozen at Job creation.** Editing the Kit file mid-Job changes nothing for any Drone of that Job, including the ones it has not spawned yet |
| 5 Task | **Frozen at Job creation** — `acceptance_criteria[]` especially, or the Judge grades against a list something invented later |
| 6 Step | Its content changes at each step boundary, because the step does. Its *sources* are frozen with the rest |
| Allowlist, budget, freeze | **Live** — re-read at every gated checkpoint. Not prompt content; enforcement |

**A scope revision is a respawn, not an edit** — and it is the only
respawn that re-resolves anything. A step boundary is a respawn too, and
it re-assembles the same six layers from the same snapshot. On a scope
revision Fleet terminates the Drone, re-resolves configuration against the
new Manifest set, and spawns a fresh one, because a person approved the
new set. **There is no path that mutates a running Drone's system prompt**
— layers 1 to 6 are fixed for the process lifetime, and a process now
lasts one step.

---

# 4a. Injected turns

**Added Aug 2026** after adversarial review. The first cut said no path
mutates a running Drone's prompt, dropping the word *system*. The dropped
word was load-bearing: Fleet injects turns into a live session, each of
which is an assembled prompt and therefore governed here.

| Turn | Fires when | Wording |
| --- | --- | --- |
| **The poke** | Liveness nudge, bounded by `poke_limit` | Drafted, not sanctioned |
| **The clarification reprompt** | Evidence arrived but was insufficient | Drafted, not sanctioned |
| **The force-interrupt directive** | A thrashing verdict | Drafted, not sanctioned |
| **The refusal reprompt** | A Judge refused a criterion after a mechanical check passed | Specified |
| **`redirect_drone`** | A structured instruction from a human, or from Helm at rung 1 | A person writes it |
| **The Pilot handoff instruction** | Fleet tells the Drone to call `escape_hatch` after a human confirms Pilot | Drafted, not sanctioned |
| **The hatch-unavailable answer** | A Drone pulled the hatch on a Job Fleet had not marked | Drafted, not sanctioned |
| **The gate's outcome** | A step passed its gate and its Drone is still there | Drafted, not sanctioned |
| **The part-before block** | The same fact, where the Drone that would have read it has ended | Drafted, not sanctioned |

**Every turn Fleet authors now has wording.** `redirect_drone` carries a
person's words rather than Fleet's, so it has none of its own to specify.
The gate's outcome was the one exception — the mechanism was decided and the
wording never written — and its last two rows close that.

**The last row is not injected into anything, and that is the point.** A Drone
belongs to a step, so at a step boundary the process that would have received
the gate's outcome has ended before the next one starts. There is no session to
inject into, and what was a turn becomes a block of the next Drone's opening
brief.

> **Rule.** A turn that has to cross a step boundary is rewritten for a reader
> with no memory of what it refers to.
> Why: the wording does not survive the move. "Go on to Implement" is a
> continuation addressed to a Drone that was there, and one that was not reads
> it as an instruction about work it cannot remember doing.

The base-moved note already worked this way and had no row: a rebase runs at
every spawn, and where there is no session it rides the brief. Both are
`fleet::crossing`, which holds every re-tensing of a live-session turn.

**One Fleet-authored turn is specified and the rest are drafted.** A draft
is wording somebody can argue with; it is not wording Armada has agreed to
send. One of these turns is the mechanism a baseline clause used to
describe, which is why that clause moved here in section 5.

**The refusal reprompt and the clarification reprompt are not one turn.**
Clarification fires on evidence that arrived and was insufficient, and
asks for the same work evidenced better. A refusal fires on evidence that
was sufficient and was judged against, and asks for the work to change. A
Drone reading one as the other resubmits the same diff with a wordier
claim, which is the failure the Judge exists to catch arriving by the back
door.

**Two properties they share.** Each arrives mid-session, so it cannot
restate context the Drone already has without spending tokens twice. And
each arrives at a moment the Drone believes it is doing something else, so
it must be unambiguous about whether the current action continues or
stops.

## The poke

Fires when nothing structured has arrived at all. `poke_limit` is a
liveness counter, distinct from the evidence clarification-round cap,
which checks content sufficiency. Both can end in `stalled`.

The turn cannot tell whether the Drone is working, finished, or stuck, so
it has to address all three without pressing any of them.

**Drafted wording. Not sanctioned.**

```
┌─ TURN ─────────────────────────────────────────
│ Nothing has arrived from you for 12 minutes.
│
│ If you are working, keep going. If you are
│ finished, submit — work you do not submit is work
│ no one sees. If you are stuck, use the escape
│ hatch. It is a sanctioned outcome and not a
│ failure.
└────────────────────────────────────────────────
```

Three branches, because the poke cannot tell which one it is in.

The escape-hatch line repeats a baseline clause on purpose: the moment a
Drone most needs to know the hatch is sanctioned is the moment it has gone
quiet.

Elapsed time survives where a count would not. Twelve minutes names no
threshold and hands over no budget. This turn must never become "second of
two pokes", which would tell a Drone precisely how long it has left to
look busy.

**This draft decides something section 5 has not.** It uses the word
*sanctioned*, and framing on the `escape_hatch` clause is deliberately
unspecified pending measurement, because opt-out rates move with the
authority the option is framed through. Sanctioned is the maximal framing
available, and this draft reached for it.

## The clarification reprompt

Asks for the same work, evidenced better. A free round that does not
consume the retry budget.

The cap has no field name and no counting scope — per Job, per workflow
step or per loop iteration is undefined. Tracked as
`[drone-evidence-clarification-cap-scope]` in
[Drone](../concepts/drone.md), and its value as
`[evidence-clarification-round-cap]` in
[Configuration](configuration.md).

The baseline clause that used to describe this mechanism lives here, since
stating it in advance spends a constant on a case most Jobs never hit.

**Drafted wording. Not sanctioned.**

```
┌─ TURN ─────────────────────────────────────────
│ Your submission arrived and there is not enough
│ in it to check.
│
│   Shown by names no artifact. "Tests pass" is not
│   one. Name the test and the command you ran.
│
│ Submit again.
└────────────────────────────────────────────────
```

**The turn says nothing about budgets.** Saying a round is free only means
something to a Drone that knows rounds are counted, and a Drone told
rounds are counted has a bar to optimise against.

Naming the report format is not naming the bar. Telling a Drone to name
its test and its command shapes the record. Telling it what the test has
to prove would shape the work, and that is the line.

## The force-interrupt directive

Stop and report current state now. Failure of that escalates as
`thrashing`.

The mid-step Judge pass that produces the verdict does not read the Drone
transcript. It judges the work product as the branch stands against the
declared scope and the step's intent, so what the turn can carry is an
observable that has not moved rather than a turn count.

**Drafted wording. Not sanctioned.**

```
┌─ TURN ─────────────────────────────────────────
│ Stop and report your current state now.
│
│   Expected   A second Job starts on its own path
│   Produced   worktree path already registered,
│              unchanged across every approach you
│              have tried
│
│ Submit what you have. Partial work with an
│ accurate Not claimed is worth more than carrying
│ on.
└────────────────────────────────────────────────
```

The mid-step record is what makes this actionable rather than a scolding.
"You are thrashing" tells a Drone nothing it can act on. An observable
that has not moved across every approach tells it which approach to stop
repeating.

Thrashing is the absence of change in `produced`. The turn count is not
the finding — it is what makes an unchanged observable mean something —
and it stays out of the Drone-facing rendering, for the same reason every
other counter does.

## The refusal reprompt

Carries the Judge record's `expected` and `produced` back to the Drone.
Never `consequence`, which is written for a person deciding whether to
care, and never a counter.

The closing line is where `what_changed` on the next work submission comes
from — [Agent Copy Contract](agent-copy.md).

```
┌─ TURN ─────────────────────────────────────────
│ Part 2 did not pass.
│
│   Expected   Suite red when a work machine's row
│              shows 68% quota left
│   Produced   Suite green on a row rendering no
│              spend at all
│
│ Address this and submit again. Say what changed
│ since your last submission.
└────────────────────────────────────────────────
```

## `redirect_drone`

Human-initiated, so its content comes from a person rather than from a
verdict. It carries no record.

## The Pilot handoff instruction

Human-initiated. The Drone's only contribution to the handoff is a
narrative of what it is stuck on, passed as the tool argument; Fleet
assembles everything else. It lands in the bundle a
[Pilot](../concepts/pilot.md) session opens with.

Framing on the `escape_hatch` clause is deliberately unspecified in the
baseline, because opt-out rates move with the authority the option is
framed through. The same caution applies to this turn.

**Drafted wording. Not sanctioned.**

```
┌─ TURN ─────────────────────────────────────────
│ A person is taking over this Job. Stop what
│ you are doing and call the escape hatch now.
│
│ It takes three things, and they are the only
│ part of the handoff that comes from you:
│
│   Trying to    what this part was meant to
│                produce
│   Blocked by   the specific thing in your way,
│                or nothing, if nothing is
│   Tried        what you attempted, and what
│                each attempt produced
│
│ Name the command, the path or the message. "I
│ am stuck" leaves nothing to act on, and a
│ blocker you did not hit leaves less.
│
│ Do not submit your work first. This is not a
│ submission and the handoff does not wait on
│ one.
└────────────────────────────────────────────────
```

**The turn offers nothing, so it frames nothing.** A Drone told to hand
over is not choosing the hatch, and framing moves an opt-out rate only
where there is an opt-out. This turn carries the caution section 5 asks
for by having no authority word in it at all, which is the one thing the
poke draft could not do.

**Nothing is a legal `blocked_by`.** Two of Pilot's three trigger paths are
human-initiated and the Drone may be fine. A Drone asked what is blocking
it when nothing is will name something, and a fabricated blocker is the
failure the hatch exists to prevent arriving through the hatch itself.

**The three fields are named and the bar for them is not.** `blocked_by`
is the field a person acts on, so the turn asks for the command, the path
or the message rather than a characterisation. What the blocker has to
amount to is left unsaid, the same line the clarification reprompt draws.

**The prohibition is specific.** *Stop* does not stop a Drone carrying a
baseline that says work it does not submit is work no one sees. *Do not
submit* does, and it is the move the M1 rendering makes on the parts that
are not the Drone's.

The tool is described rather than named, as it is in the poke and in the
baseline clause. And the turn carries no record: nothing observable fired
it, there is no verdict behind it, and the bundle already holds every fact
Fleet could put in one.

## The hatch-unavailable answer

Fires when a Drone calls `escape_hatch` on a Job Fleet has not marked for
the handoff. The pull does not go through and the Job escalates as
`hatch_unbidden` — [Pilot](../concepts/pilot.md) owns the mechanism.

**Not the refusal reprompt.** That one carries a Judge verdict on evidence
that arrived. This one answers a tool call, and no verdict sits behind it.

**Drafted wording. Not sanctioned.**

```
┌─ TURN ─────────────────────────────────────────
│ The escape hatch is unavailable.
│
│ Submit what you have through the evidence
│ submission tool you have been given, and say
│ what you did not reach. Name the command, the
│ path or the message that stopped you.
│
│ Partial work with an accurate Not claimed is
│ worth more than carrying on.
└────────────────────────────────────────────────
```

**The turn says the hatch is unavailable and nothing else.** A refusal
that explains itself teaches the Drone that a mark exists, and a Drone
that knows a mark exists has a gate to reason about rather than work to
do.

**It carries no record and no counter.** Nothing observable fired it — the
Drone's own tool call did, and the Drone already knows what it tried.

**It asks for the bar `blocked_by` asks for**: the command, the path or
the message rather than a characterisation. Submission is the action left
to the Drone, and the closing line is the force-interrupt directive's,
because what it asks the Drone to do is the same.

---

# 5. The Drone baseline

**Revised Aug 2026** after adversarial review. The first cut carried eight
clauses. Three belong here, four belong where their mechanism lives, and
one is deleted.

## The membership test

**A baseline clause must name an action the Drone can take or decline to
take. A clause that names no action fails.**

This is the acceptance test the first cut lacked, and it is the
counterpart to the [Agent Copy Contract](agent-copy.md)'s substance
requirement — a content rule rather than a shape rule, so it survives P6,
and checkable, so section 5 can be wrong.

A second test decides between the baseline and per-Job assembly: **does
the clause name a mechanism the Drone must reach for, or describe a system
property it cannot act on?** A mechanism the Drone cannot discover any
other way belongs in layer 1. A property belongs beside the thing it
describes, where it is timely.

## Why the baseline is small

Instruction-following degrades non-linearly in the number of simultaneous
instructions — measured elsewhere at roughly 96% down to 20% as stacked
verifier-checked instructions rise from one to twenty, with
inter-instruction tension a principal driver.

The exposure is worse than the clause count suggests, because layer 1 is
only the first of six. A Drone on a Bug step carries the baseline, plus
Voice, **Way I work** and **Expectations**, plus Manifest conventions,
plus WorkflowDef framing, plus `acceptance_criteria[]`, plus the step's
criteria and Evidence Scope policy, plus the exemplar corpus on prose
steps. **Section 3 bounds none of it.** A constant prepended forever to an
unbounded stack is the thing to minimise, which is why four clauses moved
out rather than staying for tidiness.

## The three clauses

| Clause | Action it names | Why layer 1 rather than beside its mechanism |
| --- | --- | --- |
| **Completion is claimed through the Evidence tool, and nowhere else.** Saying a step is done in prose does not advance anything | Call the tool | The v1 failure. A Drone that does not know the sanctioned path invents one, and it needs to know before the first step ends |
| **Stopping and handing back is a legitimate way to finish. Guessing is not.** `escape_hatch` ends autonomous execution and passes the Job to a person | Call `escape_hatch` | The moment it is needed is the moment the Drone has stopped reading the step, so no per-Job assembly gets it in front of the Drone reliably |
| **A denied command is denied, not an obstacle to route around.** Denials are silent, so notice them rather than continuing without the capability | Stop and report on a denial rather than proceeding | `--permission-mode dontAsk` denies without a prompt. Nothing else in the prompt would ever mention a denial, and the measured v1 failure is work continuing past one |

**Framing is deliberately unspecified on the `escape_hatch` clause.** The
abstention literature finds opt-out rates move materially with the
authority through which the option is framed, and fall as that authority
falls. "Sanctioned" is the maximal framing available.

The clause specifies the content; the authority word waits on
measurement. The poke draft in section 4a reaches for *sanctioned*
already, which decides in a draft what this section leaves open.

**That is incoherent as it stands, and there are three ways out.** Not
picked here — a person picks.

| Way out | What it costs |
| --- | --- |
| **The poke draft drops the word** | The turn loses its framing at the moment 4a says framing matters most — a Drone gone quiet. Nothing else. Reversible in a line |
| **The baseline adopts it** | Answers the open question from a draft, at the top of the range, where a Drone bails on work it could have done. Layer 1 is compiled in, so revising it is a build |
| **The measurement settles it** | Runs, and the time to design them. Blocks nothing while it is pending, provided the draft is not standing in for the answer |

**The contract's own reasoning favours the third, with the first as its
interim.** Three things in this document point the same way. Section 5
states the question is open pending measurement, and a draft is not where
an open question gets answered. Layer 1 is unreachable from config, so a
word chosen there is the most expensive one in the stack to change.
And the baseline may carry no claim about consequences that is not true —
an authority nobody has established is exactly such a claim.

**The third has now been run.**
[Spike 7](../spikes/007-will-a-drone-take-the-escape-hatch.md) measured
the word against a task that cannot be done, on a fixed resolution
declared before the runs. Framing moved abstention from half to all of
them, monotonically in the height of the word, at no bail rate the design
could detect on work that could be done. It cannot separate *legitimate*
from *sanctioned*, so the milder word is not a choice against evidence.

**The measurement supports the word. It does not sanction it** — that is
still a person's, and the clause and the poke draft stay marked as drafts
until one says otherwise. Two things the spike found bear on this section
directly. The mechanism sentence on its own moved nothing measurable,
which makes the clause the carrier and the word the payload rather than
two separable decisions. And [Pilot](../concepts/pilot.md)'s mark now
turns an unbidden pull into an escalation, so the quantity this section's
premise is written about — an opt-out rate — is one the mechanism no
longer produces.

## The four that moved, and the one deleted

| Clause | Now lives | Why not the baseline |
| --- | --- | --- |
| Insufficient evidence gets a clarification round, not a failure | **The clarification reprompt itself** — section 4a | Describes what Fleet will do. It is timely at the moment Fleet does it, and stating it in advance spends a constant on a case most Jobs never hit |
| Work stays inside the declared `context_paths` | **Layer 6**, per step, per `context_source` | `context_paths` is a Drone **output** under `drone_declared`, not an input. The obligation is to declare, and when — which is `declare_plan_at`, a step field |
| Secrets are brokered and never held | **Baseline — retained** | The review sited this at layer 3 with the brokered scope. Kept here instead: a Drone that does not know secrets are brokered may search the repo for one **before** reaching any secret-needing step, so the clause has to precede its own mechanism. It names an action — stop and report rather than search |
| Fleet verifies independently. Self-report is a signal, never the source of truth | **Nowhere, pending measurement** | Names no action, and the counter-hypothesis is at least as plausible: a model told an independent verifier will re-derive the truth has less reason to invest in evidence, not more. The Judge is blind to the Drone, so better prose cannot influence the verdict anyway — which makes spending tokens on the diff instead the **rational** response to being told this |
| The allowlist is not a sandbox | **Deleted** | Names no action and offers none. A Drone has no channel by which to establish that an allowed operation is safe — worktree confinement is enforced by the filesystem and by a VCS type with no push method, neither of which a Drone can consult. The sentence it derives from is addressed to a human designer |

**What the baseline must not contain.** No shape rules — P6 applies to
input as much as output, and a structural instruction gets satisfied
identically forever. No role-play framing. No claims about consequences
that are not true. **And no clause that fails the action test**, which is
what deleted the sandbox line.

**A caveat this section must carry about itself.** A baseline written as a
numbered list of rules is the shape a model pattern-matches and then
satisfies formally. The table above specifies content; the wording is
below, and whether the same content lands better as prose or as worked
examples is unmeasured.

## The M1 renderings

**The Kit and Manifest layers are absent, because M1 has neither.** The
six-layer order in section 3 is unchanged. Six decisions fell out of
writing these, and each is visible in the text.

**The reporting clause is load-bearing, not a reminder.** Given one MCP
tool and a task, told nothing about reporting, a Drone fixed the code and
wrote a tidy sentence saying so — four times out of four, no tool call.

A Drone does not reach for the Evidence tool on its own, so the baseline's
second paragraph is the difference between a working gate and a Job that
stalls on every step. Measured in
[spike 6](../spikes/006-will-a-drone-use-the-evidence-tool.md).

- **The tool is described, not named.** "The evidence submission tool you
  have been given" — the MCP tool's own description carries its name, so
  the prompt and the tool cannot drift apart.
- **"Parts", not "steps".** Deliberate: `step` is Armada's word for a plan
  artifact, and a Drone that learns the system's vocabulary can reason
  about the machinery. This orients it without teaching it the schema.
- **The stop sits inside the list, not after it.** Where the line falls
  *is* the boundary. Later parts are marked not-yours with the specific
  prohibition, because "do not run it" prevents a concrete thing that
  "stop here" does not.
- **The part before is quoted *and* its file is named.** Quoting alone
  hands the next Drone a sentence a Drone typed about a document, which is
  what a step's product being a real file exists to stop. Naming the path
  alone spends a tool call to read two lines. Where the earlier part
  declared no file — most steps, whose product is the diff — the block
  points at the branch instead, and no path is invented.
- **An earlier part with nothing recorded says so.** The two absences are
  not the same absence: on part 1 the block is not rendered at all, because
  the rail directly above marks nothing done and there is nothing an
  absence could be read as answering. On a later part the rail *does* mark
  the earlier one done, so silence there reads as a block that was
  answered, and it is spoken instead.

The rule they all serve — that a Drone is never told what the Checks are —
is on [Drone](../concepts/drone.md), since it governs every Drone-facing
surface rather than only these samples.

### Bug, part 2 of 4

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
│ It wrote that part's finding to
│ .armada/artifacts/plan.md, in the worktree you are
│ in. Read it before you start. What is quoted above
│ summarises it and does not replace it.
│
│ Parts 3 and 4 happen after you submit, and doing them
│ yourself does not move this task forward. Leave the
│ branch in a state they can start from.
└──────────────────────────────────────────────
┌─ THE PART BEFORE THIS ONE ─────────────────────
│ Plan the change passed the checks that gate it, and
│ its work is on the branch you are in. It is settled:
│ it is not yours to do again, to review or to improve
│ on. Start this part from it.
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

The step's closing line is where the work submission's `not_claimed` field
comes from — an adjacent problem noticed and left alone has somewhere to
land.

### Code Review, part 2 of 3 — the inverted case

The test of the layering, since the diff is the Drone's **input** rather
than its output.

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

**What the inverted case forced.** A fifth block — the diff is injected as
its own section *after* the step instructions, because it is reference
material rather than an instruction. And `Do not edit the code you are
reviewing`, which stops a review Job quietly becoming a fix Job: a coding
Drone's instinct on seeing a diff is to improve it.

Everything above the step block is structurally identical between the two
samples. Only the step text and the injected material differ, which is the
layering doing its job.

---

# 6. Exemplars over rules

Established on the [Agent Copy Contract](agent-copy.md) and restated here
because it is a prompt-assembly rule: **seeding a prompt with real samples
beats adding another rule.** Measured in the field — AI-generated
templates used words the real author had never once used, and the fix was
samples rather than a rule.

This is the answer to a failure a blocklist structurally cannot catch. A
clause welded into a noun generates a new string every time, so ban the
grammar, not the phrase — and show the grammar you want rather than
describing it.

## Resolved at layer 3, injected at layer 6

**Corrected Aug 2026.** The first cut sited the corpus at layer 3 and left
it there, which meant paying for it on every step of every Job forever.

The corpus is repo-specific in **origin** and step-specific in **use**.
Roughly 50 curated commit messages and 10 PR bodies, drawn from pre-AI
history — but the surfaces it exists to seed are written at `merge` and at
whichever step opens the PR. On Refactor's six steps it is live context
for one. Ten PR bodies is not a small constant, and layer 3 is frozen at
spawn and spans the whole Job.

So: resolve it at layer 3, where it comes from. Inject it at layer 6, on
steps whose evidence type is a commit message or a PR body.

**Eight is a legal size.** The Copy Contract's fallback: if the pre-cutoff
corpus is thin or poor, hand-write 8 to 10 exemplars. About an hour, and
arguably better, since the awkward cases — a revert, a partial fix, a
change that turned out wrong — can be covered deliberately. Fifty is a
target, not a floor.

**Convoy resolution is unstated.** The corpus resolves at layer 3, and
layer 3 resolves across several Manifests on a Convoy. Under *knowledge
unions* a three-workspace Convoy carries 150 commit messages and 30 PR
bodies. Nobody has said whether that is intended.

Cutoff date decided — found by inspection when the corpus is harvested,
with the method recorded alongside it.

---

# 7. What is never injected

| Never reaches | What | Enforced by |
| --- | --- | --- |
| Any invocation | Secrets, **at assembly**. `Secret<T>` has no `Debug`, `Display` or `Serialize`, so a prompt Armada builds containing one does not compile | Type system — but only for text Armada assembles. A credential sitting in a repository file is a plain String in the Drone's context and the type system is not involved. The three-sink `Redactor` and worktree confinement cover what it cannot reach. **Neither is a guarantee that a Drone's context is secret-free** |
| The Judge | The Drone's transcript or self-report — constitutional rule 2 | `verification` assembles the prompt; the Drone never touches it |
| A Judge in a panel | The other judges' verdicts — rule 5. Independence is what makes unanimity mean anything | Dispatch |
| A Drone | Another Job's context, evidence or transcript | One worktree per **Job**, which every Drone of that Job and no other shares; Fleet assembles per step, from that Job's record |
| Helm | Anything outside the selected Manifest | Fleet MCP scoping |
| A Drone | The operator's own MCP servers. v1 spawned without `--strict-mcp-config` and got 103 tools | `DroneSpawnConfig`, non-optional |

---

# 8. Where prompts live

**The invariant, which is decidable today:**

> **Every verdict must be reproducible from recorded state.** Whatever
> prompt produced a Judge refusal must be identifiable from the Evidence
> row, by version or by content hash. A prompt home that cannot satisfy
> this is not a candidate.

That eliminates at least one candidate before the question is answered —
an unversioned file under the Kit or Machine config directory — and it
applies to the baseline as much as to the Judge. Two Jobs run a week apart
against different baselines are not comparable, and nothing currently
would say so.

**What is unresolved.** `judge_check.prompt_key` is specified as a
reference into a prompt library and the library is sited nowhere. The
layer model implies homes for layers 1 to 4, and says nothing about where
a named Judge prompt is stored or how it is versioned. See Open questions
below.

---

# 9. Known gaps

**Raised by adversarial review, Aug 2026**, and not resolved here.

| Gap | Section |
| --- | --- |
| **Tool documentation.** How a Drone is shown its Commands and MCP tools, and with what descriptions. Commands, MCP and Sub agents are frozen for the Job and this contract names no wording for any of them. Tracked separately | 2, 3 |
| **No token budget** for the assembled prompt. Six layers plus a corpus, unbounded | 3 |
| **Wording for the injected turns.** Every turn Fleet authors is drafted rather than sanctioned, save the refusal reprompt | 4a |
| **The `Commit/PR message template` collision.** A Manifest template is layer-3 prompt content and it is a shape rule. Section 5 forbids shape rules in the baseline and says nothing about layers 2 to 6. Precedence against the copy lint is undecided | 3, 5 |
| **Prior-iteration context on loop workflows.** Evidence Scope appends it, which is a further source of prompt content — and for the Judge it sits oddly against "stateless, one-shot, no memory between calls" | 2, 3 |
| **Helm has no frozen/live story.** Sessions are persistent and per-Manifest; Voice and authority are configurable; Helm has no respawn path. What happens to a live session when the Kit or Machine config changes is unstated | 4 |
| **Two of the three Job shapes have no name** to classify into. Inherited from System Architecture | 2 |
| **Criterion independence** — may a Judge grading criterion 2 see criterion 1's verdict? | 2 |
| **What a Job summary is and who writes it** | 2 |

---

## Open questions
- Where does the prompt library live, and is a prompt version recorded on the verdict?

Also bearing on this document, and written where each belongs: `[manifest-scanner-agent-or-model-call]` in `system-architecture.md`. A question has one home — answering it in two places is how one of them goes stale.
