# Agent Prompt Contract

**Kind:** contract. **Governs:** text injected into a model — every prompt
Armada assembles and puts in front of one. The prompts themselves are
catalogued separately, one entry per prompt. Sibling to the
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

Every row in Armada Prompts passes this test — the invocations below, plus
the injected turns in section 4a.

**Not in scope:** what a model writes back — [Agent Copy
Contract](agent-copy.md). The task text itself, which comes from the Job.
Model selection and budget, which are per-step configuration.

---

# 2. The invocations

The invocations are rows in Armada Prompts, filtered to `Kind =
Invocation`. What each is told and never told is in the row. Three of the
things [System Architecture](system-architecture.md) section 9 lists are
not rows here:

| Section 9 lists | Here |
| --- | --- |
| Clarification round | **An injected turn** — section 4a. A separate assembled prompt, not a separate invocation |
| Sub-agents inside a Drone | **Not assembled by Armada.** Claude Code assembles it from Kit and Manifest Sub agent definitions, which are layer 2 and layer 3 content this contract governs. Armada supplies the definition, not the prompt |
| Denial-pattern suggestion | **Possibly not a model call.** If it is, it is a row. Unresolved on section 9 and inherited here |

**Voice generation is not among them**, for the reason below. It used to
carry a row that said so, which double-counted the thing the row existed
to exclude.

**Two carry a toolset.** Drone and Helm are Agents; the Judge and the
classifier are model calls. The scanner is the open case. See
[System Architecture](system-architecture.md) section 9.

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

**Voice is frozen at spawn** for a Drone, alongside Skills, MCP, Agent
files, Sub agents and Commands — changing the setting mid-Job changes
nothing for a running Drone.

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
assembled from six sources, and **freeze time orders them.** Not
preference — a mechanical constraint that already exists on Drone:
injected MCP servers cannot be swapped and the system prompt cannot be
rewritten mid-session without a kill and respawn. A block that cannot be
rewritten mid-session must precede the block that is rewritten every step.

| # | Layer | Source | Freezes at | Contains |
| --- | --- | --- | --- | --- |
| 1 | **Baseline** | Armada, compiled in | Build | Section 5. Identical for every Drone, every Job, every Manifest |
| 2 | **Kit** | The Kit file | Spawn | Voice and tone, **Way I work**, **Expectations**, machine-wide Skills, the Agent file, Sub agent definitions. **Voice classifies as Machine rather than Kit** — see the recorded disagreement above |
| 3 | **Manifest** | `armada.yml` | Spawn | Project conventions, repo-specific Skills, project Sub agents, the exemplar corpus (resolved here, injected at layer 6 — see section 6) |
| 4 | **WorkflowDef** | The Job's type | Spawn | What this kind of work means and what finishing it looks like |
| 5 | **Task** | The Job | **Job creation** — earlier than spawn | The request, plus `acceptance_criteria[]` with each entry's source (`ticket` / `helm_drafted` / `human`) |
| 6 | **Step** | Current workflow step | **Never** — re-assembled at each step boundary | The step's criteria, the Evidence Scope **policy**, the exemplar corpus on steps that write prose |

**Task moved from last to fifth, Aug 2026.** It freezes at Job creation,
earlier than everything above it, and sat last. Under one ordering
principle it belongs ahead of the only re-assembled layer. Two
consequences: the frozen block is now contiguous, and layer 6 is the only
thing re-emitted at a step boundary — which is the cheapest place for
re-assembly to sit.

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

| Layer | State |
| --- | --- |
| 1 Baseline | **Frozen at build.** Compiled in, unreachable from config |
| 2 Kit, 3 Manifest, 4 WorkflowDef | **Frozen at spawn.** Editing the Kit file mid-Job changes nothing for a running Drone |
| 5 Task | **Frozen at Job creation** — `acceptance_criteria[]` especially, or the Judge grades against a list something invented later |
| 6 Step | Re-assembled at each step boundary. The only layer that is |
| Allowlist, budget, freeze | **Live** — re-read at every gated checkpoint. Not prompt content; enforcement |

**A scope revision is a respawn, not an edit.** Fleet terminates the
Drone, re-resolves configuration against the new Manifest set, and spawns
a fresh one. **There is no path that mutates a running Drone's system
prompt** — layers 1 to 5 are fixed for the process lifetime.

---

# 4a. Injected turns

**Added Aug 2026** after adversarial review. The first cut said no path
mutates a running Drone's prompt, dropping the word *system*. The dropped
word was load-bearing: Fleet injects turns into a live session, each of
which is an assembled prompt and therefore governed here. The turns are
rows in Armada Prompts, filtered to `Kind = Injected turn`.

**A row's Wording column is the gap, and the test fails on it.** That is
the gap the first cut hid rather than created — and one of these turns is
the mechanism a baseline clause used to describe, which is why that clause
moved here in section 5.

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
falls. "Sanctioned" is the maximal framing available and the first cut
reached for it by default. The clause specifies the content; the authority
word waits on measurement — see Open questions below.

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
satisfies formally. The list above specifies content, not wording, and
whether the same content lands better as prose or as worked examples is
unmeasured.

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
| A Drone | Another Job's context, evidence or transcript | One worktree per Drone; Fleet assembles per Job |
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
| **Tool documentation.** How a Drone is shown its Commands and MCP tools, and with what descriptions. Commands, MCP and Sub agents are frozen at spawn and this contract names no wording for any of them. Tracked separately | 2, 3 |
| **No token budget** for the assembled prompt. Six layers plus a corpus, unbounded | 3 |
| **Wording for the injected turns.** Carried per turn in Armada Prompts. A row with no wording is the gap, and the test fails on it | 4a |
| **The `Commit/PR message template` collision.** A Manifest template is layer-3 prompt content and it is a shape rule. Section 5 forbids shape rules in the baseline and says nothing about layers 2 to 6. Precedence against the copy lint is undecided | 3, 5 |
| **Prior-iteration context on loop workflows.** Evidence Scope appends it, which is a further source of prompt content — and for the Judge it sits oddly against "stateless, one-shot, no memory between calls" | 2, 3 |
| **Helm has no frozen/live story.** Sessions are persistent and per-Manifest; Voice and authority are configurable; Helm has no respawn path. What happens to a live session when the Kit or Machine config changes is unstated | 4 |
| **Two of the three Job shapes have no name** to classify into. Inherited from System Architecture | 2 |
| **Criterion independence** — may a Judge grading criterion 2 see criterion 1's verdict? | 2 |
| **What a Job summary is and who writes it** | 2 |

---

## Open questions
- Where does the prompt library live, and is a prompt version recorded on the verdict?
- What is the actual wording of the Drone baseline prompt?

Also bearing on this document, and written where each belongs: `[manifest-scanner-agent-or-model-call]` in `system-architecture.md`. A question has one home — answering it in two places is how one of them goes stale.
