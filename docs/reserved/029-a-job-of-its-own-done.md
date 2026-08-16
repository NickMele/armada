---
id: 029
title: A Job says what done means
status: RESERVED
module: fleet
raised: an idea while a Job ran, 2026-08-16
---

# 029 — A Job says what done means

**Raised in his words:** *"when spawning we should be able to customize the done criteria. Even
in specific workflows. I think Claude Code calls this a goal. We should be able to specify a
custom goal (done) for the job."*

## What decides done today

Nothing on the Job. A Job is done when its **workflow's** last step passes and the workflow
`ends_at: branch`; `ends_at: human` stops and asks instead. The workflow is chosen by
classification, so two Jobs on the same workflow have the same idea of done however different the
tasks were.

That is the gap. *"Add rate limiting"* and *"work out why the parser drops CRLF"* can both
classify as `feature`, and only one of them is finished when a branch exists.

## The trap this must not fall into

**A goal is prose, and [`PLAN.md`](../PLAN.md) §14.3 exists because prose is not evidence.** A
step advances when its predicate holds *and* the verdict carries evidence an external command
produced — *"an agent asserting that tests pass is not evidence"*. A free-text goal handed to a
Drone to self-assess is precisely the false pass the whole gate design was built to prevent, and
it would arrive wearing the word the reader trusts most.

So a custom done has to resolve to something answerable. Three shapes, and they are not
exclusive:

| Shape | What it means | Honest? |
|---|---|---|
| A **predicate**, from the eight that exist | `--done check_passes:api:e2e`, `--done artifact_exists:docs/rfc.md` | yes — this is what the gate already does |
| A **question**, asked of you | the goal's prose becomes the `human_approves` text, so you are the evidence | yes, and it is what `ends_at: human` already is |
| A **judgement**, asked of a model | a Drone or reviewer reads the goal and decides | **no** — this is [`PLAN.md`](../PLAN.md) §14.3's refusal, and it must not be the default |

**The first two are buildable now and need no new predicate.** The third is what a reader will
ask for, and the reason to write this down before building is so the answer is *"here is why
not"* rather than a shrug.

## Where it is declared

Three places want it and they do not conflict:

- **`armada fleet spawn --done <predicate>`** — this Job, this once.
- **A workflow step** — already has `verify: { must: … }`, which *is* a done criterion. The
  extension is a workflow-level `done:` for the terminal condition rather than a per-step one.
- **A repository** — `armada.yml` saying what *finished* means here, so every Job in this repo
  inherits it. This is the one worth thinking hardest about: it is the manifest's business what a
  check is, so it is plausibly the manifest's business what done is.

## What it interacts with

[`016`](016-what-the-gate-cannot-prove.md) just landed, and a sub-Job's done is its own
workflow's. A parent gating on `subjob_passed` is asking *"did the child reach its done"* — so a
per-Job goal on a child is a thing a parent's step would have to be able to set, or deliberately
not. Decide which; a child that quietly redefines done is a gate that proves nothing.
