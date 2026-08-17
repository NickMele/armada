---
name: reproduce-failure
description: Write a test that fails for the reason the bug report describes, before anything is fixed. Use for the `reproduce` step of the `bug` workflow.
---

# Reproduce it first

You are the first step of the `bug` workflow, and the only one whose output is a **failing**
test. Nothing after you is allowed to start until the failure is captured, because a fix for a
bug nobody reproduced is a change that closes green having proved nothing.

> Copied into your guild by `armada guild init`. It is yours from that moment —
> `armada guild edit skills/reproduce-failure/SKILL.md` changes it.

## Your gate is `failing_test_exists`, and it is stricter than it sounds

Two things have to be true at once, and Armada checks both:

1. A test of the name the Job was given is **in the tree** — found by a fixed-string search that
   includes untracked files, so the test you just wrote counts.
2. `armada manifest check` **exits non-zero**.

**What it does not prove is that your test is why the suite is red** (`docs/reserved/016` §1).
The two facts are checked separately and the link between them is not. So a repository that was
already failing will satisfy this gate with a test that passes — which is the exact false pass
the predicate exists to prevent, arriving from the other side.

**So run the check before you write anything.** If the suite is already red, say so through
`mcp__armada__fleet_ask_human` and stop: you cannot reproduce a bug in a repository that is
broken for other reasons, and proceeding would record a green fix over a red tree.

## Reproduce the report, not your theory of it

Write the test that fails the way the report describes. The temptation is to find the bug first
and then write a test around your diagnosis — and a test written from a theory passes when the
theory is wrong.

- Use the inputs the report gives. If it says the dry run printed `CREATED` and made nothing,
  assert on that output, not on an internal function you suspect.
- Assert on the observable failure. A test asserting a private helper returns `None` proves your
  reading; a test asserting the command lied to a person proves the bug.
- **One test, on the narrowest thing that fails.** A test that exercises half the system fails
  again for unrelated reasons later and gets deleted.

## Prove it fails for the right reason

Run it and read the failure message. *"assertion failed"* with the wrong left and right is a test
that fails by accident, and it will pass again the moment somebody changes something unrelated.

**Then check it fails without the bug's cause and passes with it removed**, if you can establish
the cause cheaply. If you cannot yet, that is fine — the `fix` step after you does that — but say
which it is.

## Do not fix it

The gate wants the tree red. A step that reproduces and then fixes in the same exchange leaves
nothing for the fix step to prove, and the record loses the one moment where the bug was
demonstrably present.

Report what you wrote and where, through `mcp__armada__fleet_report`, and stop.
