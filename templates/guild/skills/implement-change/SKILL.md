---
name: implement-change
description: Make the change the plan describes, and get the workspace's checks green. Use for the `implement` step of the `feature` workflow and the `fix` step of the `bug` workflow.
---

# Make the change

This is the step that writes code. A plan was approved or a failing test was written; your job is
to make it true and to leave the checks green.

> Copied into your guild by `armada guild init`. It is yours from that moment —
> `armada guild edit skills/implement-change/SKILL.md` changes it.

## Your gate is `check_passes`, and you can run it yourself

`mcp__armada__fleet_check` runs this workspace's checks **in your worktree** — the same command
the gate runs to decide whether your step passed. So there is no guessing:

- **Run it before you report `done`.** A failure there is a failure at the gate, and the gate
  costs you an attempt.
- **Pass `fix: true` to run each check's fixing form** instead of its checking form. That is how
  formatting is repaired; do not reach for a formatter by hand.
- `armada` and `arm` on the shell are denied to you and always will be — the CLI writes the
  machine-global store and a Job must not reach it. `fleet_check` is your route to the same
  checks without it.

**If the gate fails, you are handed the failing check ids, their log paths and the last forty
lines of each log.** Read them. The error is in the message.

## Follow the plan, and say when it is wrong

The plan was reviewed and approved. Deviating from it silently means the thing that was approved
is not the thing that got built.

But plans are written before the code is touched, and some of them are wrong. When you find that:

- **A small correction** — a signature that differs, a helper that already exists — is yours to
  make. Report it through `mcp__armada__fleet_report` so the record says what changed and why.
- **A change to the approach** is not yours. That is `mcp__armada__fleet_ask_human`: the plan was
  approved on its merits and a different plan needs the same approval.
- **A question the plan marked `NEEDS THE OWNER` and nobody answered is not yours either.** It is a
  decision the planning step deliberately did not take, written down so that somebody else would take
  it. If the plan was approved and the answer never came, that is not consent by silence — say so and
  ask, naming the question so the reader does not have to find it.
- **A defect in Armada itself**, or in the repository's own configuration, is
  `mcp__armada__fleet_propose`. It writes an inbox entry and changes nothing.

## Match the code that is already there

Read the neighbouring code before writing. Comment density, naming, how errors are constructed,
how tests are named — all of it is a house style you are joining, not deciding.

**Every new test must be shown to fail without your change.** Write it, watch it fail for the
right reason, then make it pass. A test written after the code often only asserts what the code
already does, which is why a suite can be large and prove nothing.

## Commit before you finish

The review step runs in a worktree of its own, checked out at your branch. **It sees only what
is committed** — uncommitted work is invisible to it, and a reviewer reading an empty diff will
say so.

Commit in coherent pieces with messages that say *why*, not *what*: the diff already says what.

## Then report `done`

`mcp__armada__fleet_verdict` with `done` means *I have stopped working on this step*. It does not
mean the step passed — the gate decides that, and it will answer you with `recorded: ATTEMPTED`
rather than a verdict, because at that moment nothing has been decided.

If you are stuck rather than finished, say `stuck` and say why. *"I stopped and here is why"* is
the one thing only you can say.
