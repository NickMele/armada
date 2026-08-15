---
id: 013
title: Secrets must resolve before the run detaches
status: BUILT
module: manifest
raised: building `check --detach`, 2026-08-15 — the flag landed and secret resolution has not, so the rule has nowhere to be enforced yet
closed: building the resolver, 2026-08-15 — resolution sits above the `--detach` branch, and the detached child reads its parent's answers off stdin
---

# 013 — Secrets must resolve before the run detaches

> **Built.** The resolver exists, and the rule this file reserved was binding
> from its first line rather than retrofitted. `armada manifest check` resolves
> every granted secret in the caller's terminal — above the `--detach` branch,
> below the `--dry-run` one — and hands the answers to the detached child on its
> stdin. **The child never invokes a provider.**

## The rule, and where it applies

[`PLAN.md`](../PLAN.md) §4.7:

> **Secrets are resolved *before* the process detaches.** `armada manifest check --detach` has
> no terminal once it is detached, so a provider that prompts cannot prompt. Resolving while
> the terminal is still attached is the difference between `--detach` working with 1Password
> and not working at all.

`--detach`'s shape decided where that had to happen. The parent resolves the
whole plan in the caller's terminal — selection, the working diff, the port
block, every argv — and then hands the run to a `setsid`'d child that adopts the
run id and carries it out. **The child has no terminal.** Secrets joined that
list: `verbs::check::resolve_secrets` runs between the `--dry-run` return and the
`--detach` return, so an attached run and a detached one resolve at the same
point in the same process, and only the *destination* of the answers differs.

## What the resolver must not do, and why the obvious design did it

The obvious place for resolution is where the value is needed: `Action::Spawn`,
in the run loop. That is right for an attached run and wrong for a detached one,
and it fails **only** in the detached case — the shape that survives review,
ships, and is then reported as *"`--detach` hangs with 1Password and works with a
file provider"*.

**That inversion is now a test rather than a paragraph.**
`crates/helm/tests/secrets.rs` runs a stub provider that refuses to answer when
`ARMADA_DETACH_RUN` is in its environment — which is exactly the environment the
detached child would have given it, and is not the parent's. A detached run
passes; the same provider run from the child's position fails by name. Deleting
the parent-side resolution turns the first test red rather than leaving it green.

The stub stands in for `op` waiting on a biometric nobody is looking at. A test
cannot wait out a hang and does not need to: the *cause* is observable and the
hang is only its consequence.

## The part that needed its own pass: how

[`PLAN.md`](../PLAN.md) §4.7 rule 5 rules out the easy answers — an inherited
variable is readable by every child of the run and by `ps` on some platforms,
which gives up the per-entry grants the whole design exists to provide, and a
file is the *".env file an agent will eventually read while debugging"*
`ARCHITECTURE.md` §1.8 forbids Armada creating. That section proposed an
inherited pipe closed after the read. **That is what was built, and the pipe is
the child's stdin.**

| channel | why not |
|---|---|
| a file the child reads | `ARCHITECTURE.md` §1.8; readable for as long as it exists, and a crash between write and unlink leaves it forever |
| Armada's own environment | [`PLAN.md`](../PLAN.md) §4.5 inherits it wholesale to every child, so every secret is granted to every child and per-entry grants are void |
| argv | world-readable through `ps`, forbidden outright by [`PLAN.md`](../PLAN.md) §4.7 |
| **the child's stdin** | a pipe between exactly two processes; the write end closes before the parent returns, so the child sees EOF and no third party holds either end |

The mechanism was already there and needed nothing new:
`RunRequest::with_stdin` exists for the compose driver, for precisely this
reason — *"the transformed document goes on stdin so it is never written to
disk"* — and `ProcessGroup::spawn` writes it and drops the handle before
returning. No new `unsafe`, no descriptor inherited past the `exec`, and the
detached child's own children get `Stdio::null()` on stdin because their requests
carry no `stdin`, so the payload cannot travel a second hop.

**Three interactions this file flagged, each settled:**

- **The log file the child already inherits.** Untouched — `StdioMode::Log`
  wires stdout and stderr, and stdin was `Stdio::null()` and is now the pipe.
- **Restart.** There is none: a detached run that dies is `DEAD` and the caller
  starts a new one, which resolves again in the caller's terminal like any other.
  A run that could resume itself would be a run resolving without a terminal.
- **What `--status` may report.** Nothing changed. `--status` reads the run
  directory, the run directory holds `EnvDelta::names()` and never values, and a
  run whose secrets it cannot see is the only kind there is.

## What else the resolver had to carry

- **A prompting provider must be able to prompt.** `StdioMode::CaptureStdout`
  plus `RunRequest::session(false)`: `setsid` creates a session with **no
  controlling terminal**, so a provider spawned the ordinary way could never open
  `/dev/tty` however its stdin was wired. That was a second, quieter version of
  the same bug and it is the reason for the new variant.
- **Failure is a named error, never a hang** — which secret, which provider,
  which exit code, and a fixed message. [`PLAN.md`](../PLAN.md) §4.7 rule 3: a
  provider's own output is never repeated, because when a provider fails there is no resolved value
  registered to scrub against, so a chatty one leaks through a path structurally
  incapable of redaction. Its stderr is *inherited* rather than captured, so
  Armada never holds it and cannot repeat it even by accident.
- **`--dry-run` invokes nothing.** Reporting what would run is not a reason to
  make somebody touch a hardware key. Nor is selecting one component: a run
  resolves only what its *selected* entries were granted.
- **`commands:` resolves too**, on the same two functions. A dispatch is
  synchronous and attached, so it has no ordering problem — but a grant that
  parsed, verified and then silently did nothing would read as protecting
  something Armada never touched. Its output is masked on the way to the
  terminal, which is what `stdio:`'s `pipe` default for a granting entry was
  always for.

**What is still not wired: `run:`.** A service's grant is resolved by nothing,
because `armada manifest up` is phase 4's and there is no long-lived child to
inject into yet. The grant parses and `config verify` checks it; the injection
lands with `up`. `owns:` deliberately takes no grant at all
([`PLAN.md`](../PLAN.md) §4.7 rule 4), which is what keeps `manifest.db` free of
secrets and secret references alike.

## One divergence, recorded rather than resolved here

**What `${ref}` is.** The schema — authoritative for the config contract — says
`<scheme>://<ref>` and *"everything after it is substituted into that provider's
`${ref}`"*, and `verify` already reads the scheme as `reference.split("://")`.
So `op://Engineering/github/token` gives `${ref}` = `Engineering/github/token`,
and a 1Password provider is written `op read op://${ref}`.
[`PLAN.md`](../PLAN.md) §4.7's example is `op read ${ref}`, which under that
reading passes `op` an argument it rejects. Nothing is lost — both spellings are
expressible and the example is the thing that is wrong — but that snippet wants
a one-line fix by whoever next has the licence to edit it.
