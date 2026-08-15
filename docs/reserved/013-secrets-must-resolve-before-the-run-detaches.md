---
id: 013
title: Secrets must resolve before the run detaches
status: RESERVED
module: manifest
raised: building `check --detach`, 2026-08-15 — the flag landed and secret resolution has not, so the rule has nowhere to be enforced yet
---

# 013 — Secrets must resolve before the run detaches

> **Not a defect today.** [`PLAN.md`](../PLAN.md) §4.7's secret resolution is not built:
> `EnvDelta::secrets` carries **names** and no resolver reads them, so there is nothing for
> `--detach` to resolve early or late. This is recorded because the rule is stated in a section
> about secrets, the place it now has to be obeyed is a section about detaching, and whoever
> builds the resolver will be reading the first and editing neither.

## The rule, and where it now applies

[`PLAN.md`](../PLAN.md) §4.7:

> **Secrets are resolved *before* the process detaches.** `armada manifest check --detach` has
> no terminal once it is detached, so a provider that prompts cannot prompt. Resolving while
> the terminal is still attached is the difference between `--detach` working with 1Password
> and not working at all.

`--detach` is now built, and its shape decides where that has to happen. The parent resolves
the whole plan in the caller's terminal — selection, the working diff, the port block, every
argv — and then hands the run to a `setsid`'d child that adopts the run id and carries it out.
**The child has no terminal.** A resolver invoked from the run loop would therefore run inside
the child, which is exactly the case the rule forbids: `op` would try to prompt for a biometric
against a session nobody is looking at, and the run would hang until its ceiling rather than
fail.

## What the resolver must not do, and why the obvious design does it

The obvious place for resolution is where the value is needed: `Action::Spawn`, in the run
loop, resolving each check's `secrets:` as it starts. That is right for an attached run and
wrong for a detached one, and it fails **only** in the detached case — which is the shape that
survives review, ships, and is then reported as *"`--detach` hangs with 1Password and works
with a file provider"*.

**The seam that makes the fix cheap already exists.** `detach` resolves everything else in the
parent for the same reason and hands the child a plan it does not have to re-derive. Secrets
join that list: resolve in the parent, pass to the child.

## The part that needs its own pass

**How.** [`PLAN.md`](../PLAN.md) §4.7's own note is that the handoff *"must not use Armada's own
environment"* — an inherited variable is readable by every child of the run and by `ps` on some
platforms, which gives up the per-entry grants the whole design exists to provide. It proposes
an inherited pipe closed after the read. That is a real mechanism and it is not this item's to
choose: it interacts with the log file the child already inherits, with restart, and with what
`--status` may report about a run whose secrets it cannot see. **It wants its own pass**, which
is what this file reserves.

## Hook

`armada manifest check --detach` works today because nothing it runs needs a secret. The first
repository that declares `secrets:` on a check and runs it detached is where this is found, and
by then the symptom is a hang rather than a design question.
