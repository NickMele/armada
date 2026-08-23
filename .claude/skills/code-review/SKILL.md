---
name: code-review
description: Review someone else's change, the way Armada's Code Review workflow will run it — read, assess, deliver. Use when asked to review a branch, a PR or a diff in this repo.
---

# Code Review

**Three steps, linear, and it lands nothing.** The inverted workflow: the diff is
the input rather than the output.

| Step | Evidence | Mechanical | Judge | Advances on |
|---|---|---|---|---|
| `read` | facts note | read note exists | — | Automatic |
| `assess` | review findings | `REVIEW.md` exists | Do these findings engage with what this diff actually does, citing specific changes — or are they generic observations that would apply to any diff? **Panel of 2**, gaming check | Judge passes |
| `deliver` | bundle | — | — | The Manifest's review-gate rule |

## The inversion, and what it changes

| | Coding workflow | Code Review |
|---|---|---|
| The diff is | the work product, the thing being judged | the **input**, the yardstick it is judged against |
| The work product is | the diff | the **review** — findings tied to specific lines |
| Gaming looks like | fabricating: a weakened assertion, a narrowed suite | **rubber-stamping**: "LGTM" on a 900-line diff |
| Branch checked out | the Job's own | **the PR's** |
| It lands | a PR | nothing — there is no merge step |

**That last row is why this is three steps rather than seven.** An inverted
workflow is much lighter: nothing is built, nothing lands, and there is no
artifact to regression-verify, so most of a coding workflow's machinery has
nothing to attach to. Expect three-ish steps from the next inverted workflow too.

## The diff is delivered, not fetched

You are given the diff. You do not go looking for it. Reading only half of a
large change is prevented by construction rather than detected afterwards — a
reviewer who chooses what to read does not have the same input as another
reviewer, and unanimity between them then means less than it appears to.

**What injection does not give, stated plainly:** a file in context is not a file
understood. Rubber-stamping is caught by the panel at `assess`, not by anything
mechanical.

## Why nobody is asked what they read

A self-reported coverage list is worthless exactly when it is needed: the
reviewer who skimmed the diff will claim to have read all of it. **Some checks
must be observed rather than reported** — and where neither observation nor
report works, the answer is to remove the opportunity to skip rather than to
detect that it happened.

Two other routes were tried and dropped, so nobody rediscovers them: watching the
filesystem is too noisy, because the process also reads configs, lockfiles and
vendored trees; and forcing reads through a dedicated tool is whack-a-mole, since
denying the built-in reader pushes the model toward `bash cat`.

## What a finding must be

Tied to changed lines. A finding that would apply to any diff is the failure mode
this workflow's gaming check names: no findings on a substantial diff, findings
not tied to changed lines, findings generic.

**A review that never opened half the diff is still not a review.**
