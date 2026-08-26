# Agent Copy Contract

**Kind:** contract. **Governs:** text written at runtime by Drones, Judge
and Helm — PR bodies, commit messages, Judge summaries, evidence
summaries, Helm replies. Sits under the
[Design System — UI & Voice](design-system.md) contract. Read before
writing or changing any lint, prompt or template governing Drone-, Judge-
or Helm-authored text.

**Purpose:** governs text written at runtime by Drones, Judge and Helm. The
[Design System — UI & Voice](design-system.md) page governs static UI
chrome and is the parent contract; everything here sits underneath it and
may not contradict it.

**Sibling:** the [Agent Prompt Contract](agent-prompt.md) governs text
going the other way — every prompt Armada assembles and injects. Same
parent, opposite direction. The exemplar corpus task below is consumed at
its layer 3.

**Why it is separate:** the Design System page is pasted into design
sessions. Prompt-lint rules for Drone-written commit messages dilute that
input without helping a design tool.

---

## Scope

The surfaces are the register below, ordered by whether the text leaves
Armada. Each carries who writes it, who reads it, what enforcement it draws,
and its samples. A surface whose Samples column reads None has no sanctioned
copy, which is what a lint asserting every surface has copy would fail on.

| Surface | Written by | Read by | Leaves | Enforcement | Shape | Samples |
| --- | --- | --- | --- | --- | --- | --- |
| Commit message | Drone | Whoever reads the history later, including people who did not ask | Yes | Hard gate | Prose | Worked |
| PR description | Drone | A reviewer deciding where to spend attention | Yes | Hard gate | Prose | Worked |
| Work submission | Drone | A person, on escalation and at review — never the Judge | No | Prompt only | Named fields | Worked |
| Escape hatch narrative | Drone | The engineer taking over | No | Prompt only | Named fields | Worked |
| Judge record — refusal | Judge | A person triaging, and a Drone on retry, each a different projection | No | Lint warning | Named fields | Worked |
| Judge record — advisory review | Judge | A human reviewer at the gate | No | Lint warning | Prose | Worked |
| Helm reply | Helm | The person, in real time | No | Prompt only | Prose | Worked |

The commit message surface is written down in the `commit-message` skill, at
`.claude/skills/commit-message/SKILL.md`, and has no section here.

Text going the other way is registered in the
[Agent Prompt Contract](agent-prompt.md) — the same split as the two
contracts.

---

## Enforcement by surface

**PR descriptions and commit messages — prompt plus mechanical lint, hard
gate.**

The lint runs in the checks runner and fails the gate. **Split by
destination, not by surface:** this text leaves Armada and is read by
people who did not ask for it, which is what earns a gate.

**A failure gets one free correction round that does not consume the
retry budget.** That is what makes a hard gate safe here. Without it a
style bounce spends `retry_count`, and a Job can escalate as
`gate_failure` because of an em dash. It reuses the existing
one-free-round mechanism for present-but-insufficient evidence.

**Judge summaries — prompt plus lint, warning only.** Same phrase list, no
gate. The text stays inside Armada, and style-scoring the verification
path risks bouncing a sound judgment for tone.

The lint catches the phrase-level tells that cluster in generated text:

- Importance puffery: "marks a pivotal moment", "a testament to"
- Weasel attribution: "experts agree", "studies show"
- Faux-insight setups: "what nobody tells you", "the part everyone misses"
- Superficial analysis: "highlighting the team's commitment to..."
- Throat-clearing openers: "here's the thing", "let me be clear"
- Binary contrasts: "it's not X, it's Y"
- Colon reveals: "the best part: it learns"
- Dramatic fragments: "That's it. That's the whole thing."
- Fake-profound endings

**Evidence summaries — prompt only. Deliberately not linted.**

Evidence needs to be checkable, not well written. Layering style scoring
onto the verification path risks bouncing substantively sound evidence for
tone, which is the wrong failure mode to introduce into a gate.

**Helm replies — prompt only.** A lint in a real-time conversational loop
is visible lag.

---

## What a lint cannot do

A phrase blocklist will not hold against **shapes**. A clause welded into
a noun ("the read-vs-write split", "the fix-the-operations angle")
generates a new string every time, so no blocklist catches it. Ban the
grammar, not the phrase.

Shape-level rules live in the prompt. The real defence is P6 from the
parent contract, restated here because it is the operative rule for
everything on this page:

> Generated text is specified by what it must contain, never by what
> shape it takes. A structural rule ("state the finding, then the
> evidence, then the recommendation") gets satisfied identically forever
> and produces twenty interchangeable paragraphs.

**The substance requirement.** Every Judge summary must cite a specific
file, line or assertion that could not appear in any other job's summary.
A summary that would read plausibly under a different job has failed.

---

## PR description

Written by a [Drone](../concepts/drone.md), read by a reviewer deciding
where to spend attention. It leaves Armada, so it draws the hard gate.

Same rule and same gate as a commit message — say what the diff cannot —
with one job a commit message does not have: telling a reviewer what needs
them.

### Bullets are legal, and they are the middle

A PR body made only of bullets has dropped the cause and the caveat, which
are the two parts a reviewer needs and the two the diff cannot supply.
Rendered markdown scanned on a screen is a different medium from `git log`.

### A heading beats a colon reveal

"Worth a look:" is a dramatic setup where a heading is the honest
structure, and the lint bans colon reveals for exactly that reason. A
heading also encodes something true — this part is a question, the rest is
a report — and it is the only structure that survives a reviewer skimming a
queue of PRs.

### Naming a leftover is not asking for a decision

A caveat needs the cost, the reason it was not fixed, and the ways forward.
A reviewer who cannot act on a caveat skims past it.

### Sample

```
Two Jobs against the same repo resolved to the same worktree
path, and the second failed inside libgit2 naming neither Job.

- add() takes a job id and builds the path from it
- Three call sites updated in dispatch.rs
- test_concurrent_dispatch covers two Jobs on one repo
- The existing single-Job tests are unchanged

## Left for you to decide

Cleanup still keys off the repo name, so a killed Job now leaves
a worktree directory nothing removes. One per kill.

Fixing that means changing the sweeper's match pattern, which is
the retention path too, so it is wider than this change. Ship
this and file the leak separately, or send it back and it goes
in here.
```

One bullet says what was left alone. A reviewer who knows the single-Job
tests were not touched does not go looking for a change that is not there.

Compare the bullets against the shape-to-avoid sample in the
`commit-message` skill: "Updated add() to accept job_id parameter" narrates
the edit; "add() takes a job id and builds the path from it" says what the
code now does. Same form, opposite substance — the `claimed` distinction
again.

---

## Work submission

Written by a [Drone](../concepts/drone.md), read by a person on escalation
and at review. The [Judge](../concepts/judge.md) never reads it.

Deliberately not linted. Evidence needs to be checkable, not well written,
and style-scoring the verification path risks bouncing sound evidence for
tone.

| Field | Holds |
| --- | --- |
| `claimed` | What the work now does, as an observable — behaviour, never a description of the change |
| `shown_by` | The artifact demonstrating it — a named test, a command and exit code, a rendered string |
| `not_claimed` | Everything the claim does not assert — the gap, and the side effect |
| `what_changed` | Attempts after the first only |

### Not circular

`claimed` is behaviour and `shown_by` is an artifact. Where both are the
same sentence — "the test passes", shown by the test passing — nothing has
been evidenced.

### Named

"Tests pass" is not an artifact. `test_concurrent_dispatch` green with
`cargo test -p vcs` at exit 0 is.

### Two kinds in one field

`not_claimed` holds both a gap and a side effect — what the work leaves
undone, and what it changed that nobody asked for. A reviewer acts on those
differently, and both belong to the reader rather than to the claim.

### Empty is legal, absent is not

A Drone saying it left nothing behind is not a Drone declining to answer.

### No gaming payoff

Padding `not_claimed` buys nothing at the gate: the Judge never reads it and
the mechanical tier does not parse it. Its only reader is a person, and a
person cannot be talked into advancing a step.

### Samples

**Feature · implement.** The record that would have made a later refusal
unnecessary — the Drone noticed the width and said so.

| | |
| --- | --- |
| Claimed | The row shows quota percent on a personal machine and dollars against the cap on a work machine |
| Shown by | Both modes render in `job_row.stories.tsx`; `npm test` exit 0, 42 passing |
| Not claimed | The column width is unchanged, and the dollar string is longer than the quota string |

**Bug · repro.** `claimed` is the symptom, not the test. "A failing test was
added" describes the change and evidences nothing.

| | |
| --- | --- |
| Claimed | A second Job against the same repo dies at worktree registration |
| Shown by | `test_concurrent_dispatch` fails with `worktree path already registered`; `cargo test -p vcs` exit 1 |
| Not claimed | The test dispatches two Jobs, not three |

**Bug · fix.** Where the Drone prompt's "do not fix adjacent problems you
notice — say so in your summary instead" lands.

| | |
| --- | --- |
| Claimed | Two Jobs against one repo now take separate worktrees and both start |
| Shown by | `test_concurrent_dispatch` green; `cargo test -p vcs` exit 0, 34 passing |
| Not claimed | The sweeper still matches on repo name, so a killed Job's directory now survives |

**Refactor.** A refactor claims an absence, which is the hardest thing to
evidence. `shown_by` has to name the tests that would have caught a change,
since a green suite proves nothing about behaviour it never asserted.

| | |
| --- | --- |
| Claimed | Manifest resolution moves behind a store with no change to what callers receive |
| Shown by | `cargo test` exit 0, 214 passing, the three `no armada.yml` cases among them |
| Not claimed | The store reads once per repo where the old path re-read per call |

**Code Review · assess.** The inverted workflow holds without a special
case: `claimed` stays what a reader can do, and the work product is the
review rather than the code.

| | |
| --- | --- |
| Claimed | An author of PR 218 has four findings to act on, each on a changed line |
| Shown by | `REVIEW.md`, findings at `worktree.rs:40`, `:52`, `dispatch.rs:88`, `:104` |
| Not claimed | The two test files in the diff were read and not assessed |

**Feature · tests · attempt 2**, after a refusal. `what_changed` is not a
rule a Drone has to remember — it is the answer to the question the refusal
reprompt asks.

| | |
| --- | --- |
| Claimed | The suite goes red when a work machine's row shows quota percent |
| Shown by | `spend_mode_render` fails on the parent commit and passes here; `npm test` exit 0, 43 passing |
| Not claimed | Empty |
| What changed | The test asserts the rendered string where it asserted the call |

---

## Escape hatch narrative

Written by a [Drone](../concepts/drone.md), read by the engineer taking
over. Prompt only.

**Not Evidence.** Evidence is proof tied to an advance gate; this states
that no proof is coming. It lands in the handoff bundle a
[Pilot](../concepts/pilot.md) session opens with.

The Drone's only contribution to the handoff. Fleet assembles everything
else.

| Field | Holds |
| --- | --- |
| `trying_to` | What the step was meant to produce |
| `blocked_by` | The specific thing preventing it |
| `tried` | What was attempted, and what each attempt produced |

### `blocked_by` is the field that has to be specific

It is the one a person acts on. "I am stuck" names nothing to unblock. A
denied command names an allowlist entry someone can change in a minute.

### The denial is reported, never routed around

That clause is in the baseline, and this field is where a Drone honouring it
puts the result.

### Sample — Bug, fix

| | |
| --- | --- |
| Trying to | Stop a second Job colliding at worktree registration |
| Blocked by | Clearing the stale registration needs `git worktree prune`, which the allowlist denies |
| Tried | Building the path three ways; each one still collides at registration |

---

## Judge record — refusal

Written by the [Judge](../concepts/judge.md), read by a person triaging and
by a Drone on retry — but each sees a different projection. Lint warning.

The record is a fixed set of named fields. Labels are Fleet's fixed copy,
where uniformity is the point, so P6 governs the values and stops governing
the shape.

| Field | Holds |
| --- | --- |
| `expected` | What should be seen, returned or recorded if the work is right — the value itself |
| `produced` | What will be seen, returned or recorded instead, as the value itself |
| `consequence` | What that difference does to whoever consumes it; the triage field |

### Same kind

`expected` and `produced` must be the same class of artifact — a rendered
string against a rendered string, a response body against a response body, a
stored row against a stored row. Where the change renders nothing, the
observable is the caller's.

### Action

`expected` names the action and its result together, never the result alone.
A value with no behaviour attached — a string, a count, a message — can be
produced by a change that does not do the work. This carries the whole
weight, because `consequence` never reaches the Drone.

### In flight

On a mid-step pass for thrashing or plan drift, `produced` is what the
branch does as it stands, and `consequence` carries the deterministic facts
that fired the trigger. The Judge never reads the Drone transcript. It reads
the branch and is told the count.

### Plan

Where the evidence is a plan, a note or a document, the observable is what a
reader can do with it. A later step consuming it makes `produced` what that
step would build; a person consuming it makes `produced` the decision they
can and cannot reach.

### One line

A field that will not fit in a line is a finding that has not been made yet.

### Not a refusal

A criterion drawing no refusal writes no prose at all. Its verdict word is
on [Design System — UI & Voice](design-system.md) under Status grammar, with
the rest of the verdict vocabularies.

### Samples

**Feature · tests · criterion refusal**

| | |
| --- | --- |
| Expected | Suite red when a work machine's row shows `68% quota left` |
| Produced | Suite green on a row rendering no spend at all |
| Consequence | An empty spend column ships as verified |

**Feature · tests · gaming check.** The gamed artifact is a terminal line
that reads as success, and quoting it is the entire argument.

| | |
| --- | --- |
| Expected | A suite run that includes this Job's tests |
| Produced | `41 passing, 0 failing`, none of them this Job's |
| Consequence | Green reads as verified and nothing was |

**Refactor · behaviour changed.** Where expected and produced are meant to
match, the same-kind rule carries everything: both halves are what the
caller receives.

| | |
| --- | --- |
| Expected | Dispatch refuses with `no armada.yml in /repos/api` |
| Produced | Dispatch proceeds against an empty Manifest |
| Consequence | An unconfigured repo runs a Job with no Checks |

**Code Review · assess.** The inverted workflow. The observable moves from
what the code renders to what the author reads, and both halves stay the
review document.

| | |
| --- | --- |
| Expected | Findings the author of PR 218 can act on |
| Produced | `error handling could be more robust`, and one more like it |
| Consequence | A review round that changes nothing |

**Bug · fix · one of three refused.** Unanimity needs no extra copy. The two
who did not refuse produce no record, so a panel refusal reads exactly like
a single one, and the count sits beside the record rather than inside it.

| | |
| --- | --- |
| Expected | N concurrent Jobs, N distinct worktree paths |
| Produced | Two Jobs work, three collide as before |
| Consequence | The reported failure returns one Job later |

**Bug · fix · thrashing.** Thrashing is the absence of change in `produced`.
The turn count is not the finding — it is what makes an unchanged observable
mean something.

| | |
| --- | --- |
| Expected | A second Job starts on its own path |
| Produced | `worktree path already registered`, unchanged since turn 9 |
| Consequence | 32 turns and three attempts have moved nothing |

**Feature · scope · plan-shaped evidence.** The note renders nothing, so
`produced` is what the next step builds from it.

| | |
| --- | --- |
| Expected | One changed row on the Job Board |
| Produced | A row, a detail column, a tooltip and a setting |
| Consequence | A settings surface nobody asked for ships |

---

## Judge record — advisory review

Written by the [Judge](../concepts/judge.md), read by a human reviewer at
the gate. Lint warning.

`gates_advancement: false`. It cannot refuse, so silence would leave it with
no output at all, and pointing a person at something is its entire job.

### Prose, plus the record for the finding

The summary answers what the diff does; the three fields carry the one thing
a person has to act on, in the same shape they read everywhere else.

### Sample — Feature, spend renders by billing mode

> On a personal machine the row shows `68% quota left`, which is the figure
> gating dispatch there and the one the scope note asked for. Both modes
> read the machine's billing mode, and the approximation mark sits on the
> dollar figure only, matching how the two numbers are sourced.
>
> One thing to look at. The column was sized to the quota string and the
> dollar string is longer.

| | |
| --- | --- |
| Expected | `~$2.40 of $20` |
| Produced | `~$2.40 of $2…` |
| Consequence | The gating number reads ten times tighter than the cap |

### Sample — Refactor, nothing wrong

The case most likely to produce filler, and the record absorbs it. A diff
with genuinely nothing to report leaves the record empty, which is the
honest output and reads as one.

> Sixteen call sites move from `Config::get()` to the store. The three cases
> that produced `no armada.yml in <path>` still produce it character for
> character, and exit codes are unchanged.
>
> One behaviour does differ.

| | |
| --- | --- |
| Expected | The Manifest re-read at each step boundary |
| Produced | Read once per repo, held for the Job |
| Consequence | A Manifest edited mid-Job lands one Job late |

---

## Helm reply

Written by [Helm](../concepts/helm.md), read by the person in real time.
Prompt only — a lint in a real-time conversational loop is visible lag.

P3 and the Helm entry under Register by surface are on
[Design System — UI & Voice](design-system.md) and are not restated here.
Two things the samples below show that the principles do not say: reading a
record is not something Helm did, so status answers are impersonal and the
natural "I checked and…" opener is illegal; and the single observation read
as a permission becomes a quota, which gets filled whether or not there is
anything to say.

### Render a record, never paraphrase it

A paraphrase is a second generation of the finding, and the version acted on
should be the one the Judge wrote. Same rule Fleet follows when injecting a
refusal.

### Helm may show counters where Fleet may not

Counters are withheld from a Drone because a counter is a target. A person
is not being graded.

### Helm confirms the target, never the action

Bridge confirms because a click is cheap; Pilot does not because your hands
are on it. Helm is the only surface where the *target* can be wrong, because
Helm resolves the reference. A named target acts immediately; an unresolved
reference is echoed first.

### Samples

**State.** No "I" — nothing here is something Helm did.

> **what's job 12 doing**
>
> Job 12 is on step 3 of 5, implement, 14 minutes in. Last wrote
> `session.rs`. Spend `~$1.80 of $20`.

**Relaying a refusal.** Attribution sits in the second sentence, never the
first, because P5 makes the step the subject. And it is *refused*, not
*rejected*, which is already a Job status.

> **why did 12 stop**
>
> Step 3 did not pass. The Judge refused on "expired tokens refresh once
> rather than per request".
>
> Expected — the token refreshes once per session
>
> Produced — a refresh on every request through the middleware
>
> Attempt 2 of 3. It is retrying now.

**Acting, named target.** The only legal "I".

> **kill job 12**
>
> I killed the drone on job 12. Step 3 of 5, 16 minutes in. Evidence from
> steps 1 and 2 carries forward if you redispatch.

**Acting, unresolved reference.** It never asks whether you are sure. It
states which job and what survives.

> **kill it**
>
> Job 12, the one we have been looking at. Killing the drone there ends step
> 3 of 5, 16 minutes in. Evidence from steps 1 and 2 carries forward.

**Acting, ambiguous.** Disambiguation is the same failure as a bad
confirmation, caught earlier.

> **kill the stuck one**
>
> Two are escalated. Job 12 stalled on step 3, and job 15 is churning on
> step 2. Which one?

**Acting, stale target.** Errors say what happened and what to do. The last
sentence is there because a named job that finished usually means you meant
a different one.

> **kill job 12**
>
> Job 12 completed 6 minutes ago. Nothing to kill. Job 14 is running against
> the same repo.

**The observation, earned.** Permitted because Helm read both refusals and
the scope note before making it.

> **why does this keep happening**
>
> Step 3 has failed twice, both on the same criterion. Both refusals name
> the middleware call site.
>
> Step 2's scope note does not mention the middleware at all. My read is
> that the scope is the problem rather than the fix, and that is mine, not
> the Judge's.

**The observation, withheld.** The harder discipline.

> **anything I should know about 12**
>
> Job 12 is on step 4 of 5, regression_verify, 2 minutes in. Nothing has
> failed on it.

**Drafting a Job.** Authorship is stated because the Judge grades against
these, they freeze at Job creation, and they are the one case where the
yardstick is not in the requester's words.

> **fix the token refresh thing**
>
> I can dispatch a Bug job against `armada`. The acceptance criteria would
> be these, and I wrote them — they are not from a ticket.
>
> 1 · Expired tokens refresh once per session, not per request
>
> 2 · A failed refresh signs the session out
>
> Approve to dispatch, or edit them first.

### The same answer, wrong

> Here's the thing — I took a look at job 12, and it's interesting: the
> pattern here suggests the scope note might be the real culprit. It's not a
> code problem, it's a planning problem.

The faults, in one sentence: a throat-clearing opener, first person for
reading, a colon reveal, a binary contrast, an unflagged inference, and no
answer before the observation. It is longer than the correct version and
contains none of the facts.

---

## Deferred: the rubric

Scoring generated text with a cheap model against a small rubric
(directness, density, specificity) is the right eventual answer to
queue-level sameness, which is invisible one message at a time and
obvious across twenty.

Not yet. It costs a model call per artifact, and comparing a PR
description against the last thirty requires thirty to exist. Revisit
once the corpus does — tracked below.

---

## Task: build the exemplar corpus

Seeding the prompt with real samples beats adding rules. Measured
evidence from the field: AI-generated templates used words the real
author had never once used, and the fix was samples rather than another
rule.

- **Find the AI cutoff in the commit history.** Do not assume a date.
  AI-written messages have a shape: conventional-commit prefix plus a
  bulleted body, or a paragraph restating what the diff already shows.
  Inspect a window either side.

```bash
git log --since=2025-06-01 --until=2026-01-01 --format='%ad %s%n%b%n---' --date=short
```

- **Target roughly 50 commit messages and 10 PR bodies.** Commit messages
  are one line. This is not a 100,000-word corpus problem, and at that
  size every sample can be read before it goes in.
- **Curate, do not dump.** The failure mode with old commits is the
  opposite of slop: half will be "wip" and "fix typo", and dumping them
  teaches a Drone to write "wip". The ones worth keeping are where
  something was actually explained, which is a small fraction.
- **Fallback.** If the pre-cutoff corpus is thin or poor, hand-write 8 to
  10 exemplars. About an hour, and arguably better, since the awkward
  cases (a revert, a partial fix, a change that turned out wrong) can be
  covered deliberately.

**Where it fits:** the M0 v1 harvest step, since `v1-archive` is already
open at that point.

**Status:** method decided Aug 2026 — the date is not chosen up front. M0
finds it by inspecting a window either side of the suspected boundary and
**records the method and the date alongside the corpus**.

---

## Open questions
- **[copy-rubric-scoring]** Should generated text be scored against a rubric by a cheap model?
  Scoring against directness, density and specificity is named as the
  right eventual answer to queue-level sameness — invisible one message at
  a time, obvious across twenty — since a phrase blocklist can't catch a
  clause welded into a noun that generates a new string every time.
  Deferred on a stated trigger rather than rejected: it costs a model call
  per artifact, and comparing a PR description against the last thirty
  requires thirty to exist first. Revisit once the corpus does.

Also bearing on this document, and written where it belongs: `[copy-lint-surface-narrowing]` in `configuration.md` — whether a Manifest may narrow which surfaces this lint covers. It lives there because the answer turns on the config direction rule, which that document records as withdrawn.
