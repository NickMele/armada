# Acceptance tests

One test per milestone, standing for the milestone's claim. It lives in
`crates/acceptance/`, it is written before the code it tests, and the gate
watches it — rule one of `cargo xtask verify-foundations`.

Read this before writing a milestone's test, reconciling one, or changing what
the gate asserts about it.

## What one is

> **Rule.** A milestone has exactly one acceptance test, and it drives the
> milestone's claim end to end.
> Why: a claim split across many tests is a claim no single run makes.

> **Rule.** The test is hermetic — no process spawned, no repository touched,
> no network opened, every adapter a fake.
> Why: a test that reaches the machine measures the machine.

The test is `crates/acceptance/tests/bug_job.rs` and the apparatus it runs on
is `crates/acceptance/tests/bench/`. The two are separated so that what the
milestone claims and what it is claimed against read as different things.

## Why it is written first

> **Rule.** The test is written before the crates it calls.
> Why: the compiler's error list then becomes the list of what those crates
> have to provide.

Written first, the test asks for the vocabulary the claim needs, and the crates
are built to that shape. Written against a finished skeleton, it is shaped by
the code it finds, and it asserts what happens to be reachable.

## Why failing was enforced, and what it cost

Through M0 — Foundations the test named APIs that did not exist, so it did not
compile, and both the gate and a session hook asserted that. Rule one was
satisfied by a non-zero exit; a Stop hook ended any session in which
`cargo test -p acceptance` came back green.

| Cost | For the whole of M0 |
|---|---|
| `cargo nextest run --workspace` | Unusable — `--exclude acceptance` everywhere |
| The acceptance crate | Uncompiled, so unchecked by every other rule |
| An agent's instinct | Fighting the gate, which is why the hook existed |

Both mechanisms are now retired: rule one asserts the test passes, and the Stop
hook and its entry in `.claude/settings.json` are deleted. The reasoning did
not retire with them. A milestone that can fake itself green proves nothing;
what changed is the direction the falsehood would run.

## Reconciling one

A test written first names an API that may be superseded while the code is
built. Reconciling is editing the test to the vocabulary that exists.

> **Rule.** A claim edited to match what was built claims nothing.
> Why: the test is the milestone's promise, not a description of the code.

Every assertion in a reconciliation is sorted into one of three, and no fourth:

| Verdict | Meaning | What happens |
|---|---|---|
| Stale | Same claim, renamed vocabulary | Rewrite against the new names |
| Right and unbuilt | Claim holds, code is missing | Build the code |
| Excluded | Claim is dropped for this milestone | Named as a change to the milestone |

An exclusion is a change to what the milestone promises. It is written down as
one, in the step that made it, and in the test's own header.

## What M1's test proves

`crates/acceptance/tests/bug_job.rs` proves the machinery of M1 — Dogfood.

| Proved | Not proved |
|---|---|
| Human approval before dispatch | The merge |
| Fleet dispatch, one submission per step | An agent doing real work |
| Evidence and passing checks both required, neither sufficient | A commit, a push, a pull request |
| Both machines — Job status and step state | The loop that drives them |
| Frozen acceptance criteria and step list | Frozen checks |
| Branch derivation from the Job id | A Judge, a retry, a human advance gate |

**The merge is the milestone's other half and no test covers it.** M1's claim
is that Armada does a small real task in the Armada repo and a person merges
the branch it wrote; the run itself is a person's to perform once. The test's
own header carries the full list of gaps.

## The next milestone

Whether the next milestone writes its test first and enforces its failure is
open. It is a decision about how that milestone is built, and neither M0's
answer nor M1's outcome settles it.

## Open questions

- **[m2-test-first]** Whether the next milestone's acceptance test is written
  before its code, and whether a gate rule asserts its failure until the
  milestone closes. M0 did both, at the cost of a workspace test run that could
  not include the acceptance crate. Nothing decides that the next one repeats
  it.
