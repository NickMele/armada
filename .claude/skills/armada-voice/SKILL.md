---
name: armada-voice
description: How Armada writes — the lexicon, the prose rules and the status grammar, for product copy and for internal documentation alike. Use before writing any user-facing string, error, status message, commit message or planning page.
---

# How Armada writes

The Design System owns this and nothing may contradict it. What follows is the
part you need in your hands while writing; read the page itself before writing
UI chrome.

**These rules cover internal documentation too**, not only product copy. The docs
are read constantly and their conventions leak into the product.

## The six principles

**Metaphor lives in proper nouns only.** Nautical vocabulary is confined to
names — Armada, Fleet, Bridge, Helm, Drone, Manifest, Convoy, Job Board. Every
verb, state, error and instruction is plain English. Write "Drone 4 stopped
reporting 12 minutes ago", never "Drone 4 has gone dark".

**Briefing register.** A message carries the facts needed to decide, on screen,
without a click.

> Weak: "Drone 4 stopped reporting. Poke limit reached."
> Correct: "Drone 4 stopped reporting 12 minutes ago after 3 pokes. Step 2 of 5, last wrote `auth/session.rs`."

**First person is Helm's alone.** Bridge and Fleet never say "I". Helm says "I"
only for what Helm itself did.

**Hedge by source.** Three source classes, three registers — and rendering any
two identically means one bad value teaches distrust of the other two.

| Source | Register | Example |
|---|---|---|
| Measured | Flat | "Tests passed." `pnpm test` exited 1 on 4 assertions |
| Estimated | Marked approximate | `~$2.40`, never `$2.40` |
| Judged | Visibly a judgment, naming its source | "Judge read the evidence as not covering the error path" |

**Event-first, with cause.** The subject of a failure sentence is the job or the
step, never the drone. Write "Step 3 did not advance. No evidence after 3
clarification rounds", not "Drone 4 failed to submit evidence."

**Fixed copy is a template; generated copy is a substance requirement.** Fleet's
own strings are identical every time, because uniformity is scannability.
Generated text is specified by what it must contain, never by what shape it takes
— a structural rule produces twenty interchangeable paragraphs. **A summary that
would read plausibly under a different job has failed.**

## Prose rules

- Sentence case everywhere. Proper nouns keep their capitals inside it.
- Name things by what the person controls: "Approve dispatch", not "Submit job payload".
- **No mid-sentence asides.** The rule targets the reflex, not the character — banning the em dash breeds a colon and banning the colon breeds a trailing negation. A colon separating a field from its value is fine: "Step 3 stalled: no evidence after 3 rounds."
- No adverbs by default. "Successfully completed" is "Completed". "Currently running" is "Running".
- No Wh- sentence openers. They survive as panel headings: "Why this stalled", "What changed".
- **No sentence that survives deletion without loss.** Remove it and see whether anything was lost.
- Errors say what happened and what to do. Never apologise, never be vague.
- An action keeps its name through the flow. A button that says Kill produces "Killed".

## The lexicon

| Term | Means | Never |
|---|---|---|
| Armada | the app | the tool, the system |
| Fleet | the daemon | the backend, the server, the sidecar |
| Bridge | the operational surfaces | the dashboard, the UI |
| Helm | the conversational surface and its agent | the assistant, the chat |
| Drone | one agent instance | the agent, the bot, the AI, Claude |
| Job | one unit of work | task, run, ticket |
| Convoy | a multi-workspace job landing as one PR | batch, group |
| Job Board | the open queue | the queue, the backlog |
| Kit | the tool set you bring | global settings, preferences |
| Machine | how this installation behaves | system settings, environment |
| Manifest | per-project config | the config, the yaml |
| Judge | the semantic verification layer | the auditor, the reviewer, AI review |
| Evidence | the structured completion report | the report, output, proof |
| Doctor | the health check | diagnostics, system status |
| Workspace | one unit inside a repo | package, module, sub-repo |

**Claude is a model name, never an actor.** "Drone 4 stalled", not "Claude
stalled". The word appears only where a model is selected or reported.

**Casing.** Docs capitalise throughout. UI capitalises the singular named things
and lowercases anything countable: "No active jobs. 3 waiting on the Job Board."

## Retired terms — a page still carrying one is stale

| Retired | Now |
|---|---|
| Guild | **Kit** and **Machine** — a split, not a rename. Each site needs judgment about which it was |
| Armada Server | **Armada API** |
| Daemon | **Fleet** |
| Ground Zero | **M0 — Foundations** |
| Phase 0 through 6, numbered implementation steps | **Milestones** and their **Steps** |

## Typography of reference

**Never use `§`.** Write "M0 step 4", not "M0 §4" — it is legal-brief typography,
it reads as affectation in a working document, and nobody says it out loud. The
same reasoning bans `¶`, `cf.`, `ibid.`, `op. cit.`, `viz.` and `q.v.` Write
"see", "compare", "same source". `e.g.` and `i.e.` are fine.

**A bare file path on an Armada page refers to v1**, and resolves against
`v1-archive` / `v1-final`, never `main`. v1 was deleted from the working tree on
2026-08-23, so a v1 citation must name the tag. Line numbers are stable against
`v1-final` because a tag is frozen.

## Status grammar

**Headline plus fields.** A headline sentence, facts as labelled fields beneath,
machine-derived fields in mono.

> **Job 12 stalled at step 3**
> Workspace `api` · 3 pokes · `auth/session.rs` · 12m · ~$1.80

**Verbs are generated from the enum, never written.** `stalled` renders
"stalled", never "went quiet". For `escalated` and `not_started` the headline
verb is the reason rather than the state, because nobody says "Job 12 escalated
at step 3". `completed_success` renders "done"; `gate_failure` renders "failed a
check"; `fan_out` renders "hit the sub-dispatch cap".
