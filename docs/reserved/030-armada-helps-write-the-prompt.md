---
id: 030
title: Armada helps write the prompt, and you approve it
status: RESERVED
module: cross-cutting
raised: an idea while a Job ran, 2026-08-16
---

# 030 — Armada helps write the prompt, and you approve it

**Raised in his words:** *"each manifest or maybe even a global armada setting should be allowed
to enable agent assisted prompts. Basically we spawn a job and before dispatching (maybe during
classification) armada can help construct the prompt to follow best practices and provide the
best opportunity for success. The human has to approve the new prompt, choose to edit or skip the
updated prompt."*

## Why this is cheap to add and easy to get wrong

**The call already happens.** Classification sends the task to Claude before anything is spawned —
measured at 9.5s on a real run — and it returns a workflow and a confidence. Improving the prompt
is more work for a turn Armada is already paying for, not a new one.

**And a bad task is the most expensive kind of failure Armada has.** A Job that misunderstands
what it was asked spends its whole budget before anybody sees the result, and everything after
that — the gate, the reviewer, the verdict — is evidence about the wrong work.

## The three-way choice is the design

*Approve, edit, or skip.* That is not decoration; it is what keeps this from being Armada quietly
rewriting what you asked for.

- **Skip has to be first-class**, and the rewritten prompt has to be shown in full beside the
  original. A diff, not a replacement.
- **`--yes` and `--json` must never take the rewrite.** A flag lives in a script for ever, and
  consent to reword a task has to be given on the run that does it — the same rule
  [`prune.md`](../commands/manifest/prune.md)'s rule 3 already applies to deleting an unlabelled
  volume.
- **No terminal means no rewrite.** Not "rewrite silently"; the Job runs with the task as typed.

[`027`](027-the-cli-under-a-users-hands.md) is directly relevant and was written the same day: its
worst finding is a tick list whose legend describes keys that do not do what it says, so it
**writes the line you declined**. An approve/edit/skip surface is exactly that shape, and would
inherit that bug if built on the same selector without fixing it first.

## Where the setting lives, and it is not obvious

He offered two and they answer different questions:

| Where | Says | Argument |
|---|---|---|
| `armada.yml` | *"tasks in **this repository** get help"* | the repo knows its own domain, and a prompt that mentions its stack is better |
| `machine.yml` | *"**this machine** does it"* | it is a spend and a latency decision, and those are per machine |
| the **guild** | *"**I** want help"* | the guild is you — voice, skills, how you work — and wanting help writing a task is a fact about a person, not about a repository |

**The guild is the one he did not name and is probably right.** `PLAN.md` §13.1's split is that
the guild is you and the manifest is the code; whether you want your wording improved is not a
property of a codebase. The counter-argument is real though: what *good* looks like is
repo-specific, so the **setting** may be yours while the **material** it draws on is the repo's.

## What it must not become

A second classifier. This rewrites the task and stops; it does not decide the workflow, pick
skills, or set budgets. Those are decided by things that can be argued with, and a prompt-improver
that quietly widened its remit would be the hardest kind of change to notice — the output still
looks like a prompt.
