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

The surfaces are rows in Armada Copy, ordered by whether the text leaves
Armada. Each row carries who writes it, who reads it, what enforcement it
draws, and its samples. A row whose Samples column reads None is a surface
with no sanctioned copy, which is what a lint asserting every surface has
copy would fail on.

Sibling to Armada Prompts, which holds text going the other way — the same
split as the two contracts.

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
