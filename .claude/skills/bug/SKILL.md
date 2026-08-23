---
name: bug
description: Fix a defect, the way Armada's Bug workflow will run it — repro, root cause, fix, regression verify, review, merge, close. Use when asked to fix a bug or investigate a defect in this repo.
---

# Bug

**Seven steps, linear.** The reference sample — it exercises every field type, and
it is the one to copy from when designing another workflow.

| Step | Evidence | Mechanical | Judge | Advances on |
|---|---|---|---|---|
| `repro` | failing test | test runs, **exits 1, status red** | Does this failing test actually represent the reported bug? | Judge passes. **Hard prerequisite** |
| `root_cause` | facts note | root cause note exists | Is this explanation plausible given the repro evidence? | Judge passes |
| `fix` | diff | diff non-empty | Does the diff address the stated root cause, not just the symptom? Panel of 3, gaming check against `root_cause` | Judge passes |
| `regression_verify` | test suite run | test exits 0 | Does the diff change behaviour beyond the stated root cause? | Judge passes; on fail, back to `fix` |
| `review` | bundle | — | Advisory summary, does not gate | The Manifest's review-gate rule |
| `merge` | — | — | — | The Manifest's auto-merge rule |
| `close` | — | PR merged | — | Automatic |

## Start with a test that fails for the right reason

`repro` is a **hard prerequisite**: it cannot be skipped and cannot be advanced
past without the artifact, **even on retry**. A bug with no reproduction is a bug
nobody can prove was fixed, and the whole workflow is built on that artifact
existing.

The mechanical check expects **exit 1**. A repro step that goes green has not
reproduced anything.

A human can override at the gate where the detection is judged wrong, and **the
override is recorded as a human action, never as a passing check** — the evidence
trail still shows the test did not fail.

## Root cause is a deliverable, not a preamble

It is judged as one: *is this a sound root-cause analysis*, never *did the worker
work hard*. It is also the baseline the gaming check compares the fix against, so
a vague note weakens the check that comes after it.

## What `regression_verify` can and cannot see

The suite's exit code catches a fix that broke something. The Judge catches a fix
that changed behaviour beyond the stated cause, by reading the diff.

**The hole, stated plainly:** a fix that breaks an unrelated test in a module the
fix never touched does not appear in the diff, so the Judge cannot see it. Only
the suite catches it — and only if that test runs in the check being named. A
repo that splits fast and slow suites and points the regression gate at the fast
one has a blind spot in both tiers. Point it at the complete suite.

## Why "no new failures" stopped being mechanical

The earlier version asserted `repro_test_now_green_and_no_new_failures` from a
single exit code. Neither half can be answered that way — both need to know which
individual tests ran and how they fared, which is parsing, and **Armada does not
parse.** The trade: "no new failures" got weaker, and became real.
