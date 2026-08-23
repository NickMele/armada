---
name: prototype
description: Build something exploratory to find out whether an approach works, the way Armada's Prototype workflow will run it — frame, build, write up. Use when the goal is to learn rather than to ship.
---

# Prototype

**Three steps, linear, and it never auto-merges.**

| Step | Evidence | Gate |
|---|---|---|
| `frame` | facts note | **None.** Fleet advances it when the evidence arrives |
| `build` | diff | **A human, always** |
| `write_up` | facts note | **None** |

## The rule this workflow exists to carry

**A step with no check and no Judge is not a failed gate — it is no gate.** Two
of these three steps have neither, and Fleet advances them outright once the
evidence of the declared type is submitted.

That matters beyond this workflow. The Judge is described as veto-only, running
only after the mechanical check holds, which read literally makes any step
without a mechanical tier unadvanceable. This is the case that resolves it, and
documentation lands the same way — which is what made it a rule rather than an
exception.

## Why there is no Judge here

Prototype is exploratory: **success criteria are fuzzy by design**, and there is
usually no firm plan for the work to be checked against. The "does this match the
plan" question every coding workflow's Judge asks has no yardstick to ask it
against.

The absent Judge tier is that absence made explicit rather than an oversight —
which is why `auto_merge: never` carries the weight instead. **A prototype does
not land on its own, ever.**

## Running it by hand

**Frame what you are trying to learn**, not what you are going to build. The
write-up is judged against the question, and a prototype that built something
impressive while answering nothing has failed.

**The write-up is the deliverable. The code is not.** It says what you learned,
what surprised you, and what you would do differently — including whether the
approach should be abandoned. A prototype that concludes "this does not work" has
succeeded.

**Do not let it become the implementation.** The human gate at `build` is the
place to say "this is enough to know", and prototype code that quietly becomes
production code is the failure this workflow's never-auto-merge rule is guarding
against.
