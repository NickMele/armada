---
name: refactor
description: Restructure code without changing what it does, the way Armada's Refactor workflow will run it — scope, refactor, regression verify, review, merge, close. Use when asked to refactor, extract, rename or restructure code in this repo.
---

# Refactor

**Six steps, linear.** The most gameable workflow in the set, and the one whose
gate is the Judge rather than the mechanical tier.

| Step | Evidence | Mechanical | Judge | Advances on |
|---|---|---|---|---|
| `scope` | facts note | scope note exists | Does this scope describe a change to structure only, with no intended change to behaviour? | Judge passes |
| `refactor` | diff | diff non-empty | Does this diff restructure code without changing what it does? | Judge passes |
| `regression_verify` | test suite run | test exits 0 | Are the changes to test files justified by the stated refactor — or has scope been narrowed, assertions weakened, tests skipped or deleted? **Panel of 2**, gaming check against `scope` | Judge passes. **Hard prerequisite**; on fail, back to `refactor` |
| `review` | bundle | — | Advisory summary, does not gate | The Manifest's review-gate rule |
| `merge` | — | — | — | The Manifest's auto-merge rule |
| `close` | — | PR merged | — | Automatic |

## Why the verification is inverted, and why that matters

**Every other coding workflow proves that something new works. Refactor proves
that nothing changed** — and a narrowed test set passes exactly as a real
refactor does. There is no positive artifact whose absence gives the game away.

That is why the panel sits on `regression_verify` rather than anywhere else, and
why the gaming check rather than the exit code is the real gate here.

## What was tried and abandoned

The first design gated on **set equality against a captured test baseline** —
identical test IDs, identical outcomes. Wrong twice over: it would have required
parsing test-runner output, and it would have blocked every legitimate refactor,
since renaming a module renames its test imports and extracting a class splits
its test file.

**This was never a mechanical question.** The test check reads an exit code. The
Judge reads the diff, and a `git diff` already carries the before and the after,
so there is nothing to capture and nothing to parse. A test file that went from
forty assertions to three is visible without anything having been counted.

## The baseline is the stated scope

Not a captured run. The gaming check compares test changes against what the scope
note said the refactor would do — *are these test changes explained by that
refactor* — rather than demanding an identical before and after that no honest
refactor could satisfy.

Which means a vague scope note disarms the only real gate this workflow has.

## What this deliberately does not do

It does not verify that the public API surface is unchanged, that no test moved
from red to green, or that performance held. Each would need a parser or a second
suite run, and both were ruled out. **If a refactor changes behaviour, the path is
the Judge noticing it in the diff, not a mechanical tripwire.**
