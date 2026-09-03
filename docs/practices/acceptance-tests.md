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

| Milestone | The claim | Where |
|---|---|---|
| M1 — Dogfood | A Bug Job runs from the approval gate to `completed_success` | `crates/acceptance/tests/bug_job.rs` |
| Focus | A Drone belongs to the step it was given | `crates/acceptance/tests/drone_per_step.rs` |
| Board | Every Job is a row, and opening one says what it did | `crates/acceptance/tests/board.rs` |

The apparatus is `crates/acceptance/tests/bench/`, shared, with a file per
milestone. Claim and apparatus are separated so that what a milestone claims and
what it is claimed against read as different things.

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

## A second milestone's test, and the one thing M0's answer cannot repeat

Focus writes its test first. `CLAUDE.md` says a milestone's test is written
before the code, so that half was never really open.

**What is decided here is that it compiles.** M0's test named APIs that did not
exist, so `cargo test -p acceptance` did not compile and rule one was satisfied
by a non-zero exit. That is unrepeatable now, for a reason that did not exist
when there was one milestone: the package holds every milestone's test and the
gate compiles all of them in one invocation. A Focus test that named `job_steps`'
Drone pointer before #137 built it would take `bug_job.rs` down with it, and
M1's claim would stop being run for the length of Focus.

> **Rule.** A milestone's test may not stop an earlier milestone's test running.
> Why: a shipped claim that quietly stops being checked is the failure the gate
> exists to prevent, arriving from the inside.

So a test written before its code is written in vocabulary that exists, against
behaviour that does not. Rule one goes red reporting a **failed assertion**
rather than a broken build, which is the more precise of the two signals it
already distinguishes — "it builds, and the claim it makes is not carried".

**What that costs is stated rather than hidden.** A claim needing vocabulary
that does not exist yet cannot be an assertion, so it is named in the test's own
header as a claim the file does not carry and why. `drone_per_step.rs` names
three. That is weaker than a compiler error list, and the compiler error list is
not available at this price.

**And one gap reports at a time.** A panic ends a run, so a test whose
assertions are in the order a Job meets them names the earliest unmet one. On
Focus that is the order the milestone is built in, so each step landing moves
the failure down the file rather than clearing it.

## What Board's test proves, and the layer it stops at

Board's claim is about a screen — every Job on a board a person can scan, and
opening one telling them what it did. Nothing in this workspace renders, so
`crates/acceptance/tests/board.rs` proves the record-to-wire seam the screen is
drawn from and names the screen as unproved. Every assertion is made against a
value that has been through `ipc::encode` and back, so it is an assertion about
what a Board receives.

| Proved | Not proved |
|---|---|
| Every Job the record holds becomes a row, whatever its status | That a person scanning the result learns anything |
| A Job that will not load rides beside the ones that did | That `store` hands back every row it holds |
| Steps, their order and state, and the gate's verdict on each | That Fleet hands the assembled facts in |
| The Judge's citation, against a criterion the Job carries | The evidence, the declared Checks, the changed files |
| What each Check did, and what a stopped Job's recourse is | Whether the work landed — #337 |

The reasons in the right-hand column are in the test's own header, and three of
them are the hermetic rule biting: `store` has no in-memory constructor, and
`fleet`'s conversions for evidence, declared Checks and a footprint are
`pub(crate)`.
