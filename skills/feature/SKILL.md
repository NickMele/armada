---
name: feature
description: Build new behaviour, the way Armada's Feature workflow will run it — scope, implement, tests, regression verify, review, merge, close. Use when asked to add a feature or implement new behaviour in this repo.
---

# Feature

**Seven steps, linear.** This is the template the other coding workflows derive
from, and running it by hand now is what makes the WorkflowDef a transcription
rather than a design. Where following it is awkward, that is a finding about the
workflow — say so.

| Step | Evidence | Mechanical | Judge | Advances on |
|---|---|---|---|---|
| `scope` | facts note | scope note exists | Does the note address what was requested, without expanding beyond it? | Judge passes |
| `implement` | diff | diff non-empty **and** build exits 0 | Does the diff implement what the scope note described, and nothing outside it? Panel of 3 | Judge passes |
| `tests` | diff | diff non-empty **and** test exits 0 | Do the tests exercise the behaviour the scope described, rather than restating the implementation? Gaming check against `scope` | Judge passes |
| `regression_verify` | test suite run | test exits 0 **and** lint exits 0 | Does the diff change behaviour beyond the scope note? | Judge passes; on fail, back to `implement` |
| `review` | bundle | — | Advisory summary, does not gate | **A human, always** |
| `merge` | — | — | — | The Manifest's auto-merge rule |
| `close` | — | PR merged | — | Automatic |

## What separates this from Bug

**The front and the gate, not the shape.** Bug opens with a hard-prerequisite
failing test because a bug has a reproducible symptom to pin down first. A
feature has a request and no artifact, so it opens with a scope note that
everything downstream is judged against.

And Bug's review reads a Manifest rule, while Feature's is `human_always` —
**new behaviour is the case where the engineer looks every time.**

## Running it by hand

**Write the scope note first, as a file.** It is not a preamble; it is the
yardstick every later step is judged against, and `tests` compares its own diff
back to it. A scope note that says "add the thing" gives the later steps nothing
to check.

**`implement` asserts two things, and one alone is not enough.** A build passes
cleanly on an empty diff, so a workflow that checked only the build would let a
run that did nothing advance. Check both.

**Declare which files you expect to touch before you start implementing**, and
notice when you leave that set. Drifting outside it is not automatically wrong —
investigation legitimately finds the real work elsewhere — but it is the signal
that earns a closer look.

**`tests` is where gaming lives.** The four patterns: an assertion weakened, test
scope narrowed, a tautological test, and the Check's own configuration edited so
a frozen command resolves to a smaller gate. The fourth is the one that hides,
because the first three are all about test code.

**`regression_verify` runs test and lint separately.** Under a single compound
check, which one failed disappears behind one exit code.

## Known open, inherited

- `on_fail: retry:implement` is a backward jump called a retry, while the
  identical jump through review's `request_changes` explicitly is not one. Whose
  retry count increments is undefined.
- `reference_docs` carries `"ticket"`, which is not a path.
