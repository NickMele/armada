---
id: 009
title: Smaller things raised in use, each with its reason
status: RESERVED
module: cross-cutting
raised: real use
---

# 009 — Smaller things raised in use, each with its reason

Recorded together because each is small; kept because each came from real use rather than
from reading the plan.

| # | Raised | What it wants |
|---|---|---|
| 1 | `doctor` says `manifest.db` is present and nothing else | Presence is not health. Row counts per table, and how much of it is stale — a store that exists and holds four thousand dead rows is a different answer from one that exists — **BUILT** |
| 2 | `config scan` offers the agent hand-over or nothing | Three options: build it with an agent, **write a blank `armada.yml` and open it in `$EDITOR`**, or stop. The middle one is missing and is what a reader who knows their own repository wants. **Half answered by [007](007-scanner-should-propose.md)**: the middle option exists and writes the *provable* config rather than a blank one, which is strictly more useful. What is still missing is opening it in `$EDITOR` afterwards — **BUILT** |
| 3 | The guild has a remote and nothing reports on it | Whether there is anything to pull, and anything local worth pushing. Same shape as `git status` and it is the same question — **already built**, at M2 |
| 4 | Sync is manual and easy to forget | An occasional offer to pull when the guild is behind — `oh-my-zsh`'s prompt is the reference. **Never automatic**: a `pull` that runs unasked can change how an agent behaves mid-session |
| 5 | `armada --help`'s `NOT BUILT YET` list has drifted from this plan | It lists `guild edit, verify`, `manifest render, agents-md, explain`, `check --detach/--status` — and nothing reserved in `docs/reserved/`. Reconciling them is a chore with a test behind it, and the test is what stops it drifting again |

**Two of these are the same rule.** #1 and #5 are both a report that has stopped tracking what
it describes. `doctor` describes a file it does not look inside; `--help` describes a plan it
does not read. Wherever a listing is derived rather than retyped — `args.rs`'s own constants
already feed the help page — it cannot drift, and that is the fix in both cases.

## What shipped

**#1.** `doctor`'s `manifest.db` row now reads through
[`armada_manifest::db::Db::peek_stats`](../../crates/manifest/src/db.rs), opened read-only so the
check stays true to "`doctor` writes to neither `~/.armada/` nor `~/.claude/`"
([`doctor.md`](../commands/doctor.md)). The table names come off `sqlite_master`, never off a
list retyped here, so a table `Db::migrate` adds needs no matching edit to the check — the same
fix #5 wants for `--help`'s list, applied to this report instead of merely described for it.
"How much is stale" reuses [`armada_core::reap::registry_pass`](../../crates/core/src/reap.rs) —
the one rule that already decides what `clean --orphaned` reaps — rather than inventing a second
opinion about what counts as reclaimable: a `workspaces` row whose directory is gone, and every
`owned` row underneath it.

**#2.** `config scan`'s hand-over now offers the write option whenever there is no `armada.yml`
yet, proposals or not — `armada manifest config scan` proposing nothing used to leave a reader
with only the agent hand-over or stopping. `config::confirm` writes the blank scaffold directly
when there is nothing to tick, and the entrypoint opens whatever was written in `$VISUAL`, then
`$EDITOR`, through `ctx.run` under `StdioMode::Inherit` — the same seam and the same interactive
mode `dispatch.rs` already uses for a `commands:` entry. Neither variable set is a clear failure
naming both, never a guessed `vi` (`crates/helm/src/verbs/config.rs`'s `open_in_editor`).

**#3.** Already built, at M2 (`08ef2cc`) — before this document was written down. `doctor`'s
`guild` check already asks git the same three questions `git status` would: `armada guild pull`
when behind, `armada guild push` when ahead, both named when diverged, `ok` when in step, and a
warning rather than a failed run when the remote cannot be reached (`crates/helm/src/verbs/doctor.rs`,
the `drift` function). `armada guild ls` deliberately leaves this alone —
[`guild/ls.md`](../commands/guild/ls.md): *"It says what is there, not what has moved. `pull.md`
already reports drift and `doctor.md` already reports what is wrong."* No code changed for this
item; it needed recording as answered rather than building.
