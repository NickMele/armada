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
| 4 | Sync is manual and easy to forget | An occasional offer to pull when the guild is behind — `oh-my-zsh`'s prompt is the reference. **Never automatic**: a `pull` that runs unasked can change how an agent behaves mid-session — **BUILT** |
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

**#4.** [`armada_guild::offer`](../../crates/guild/src/offer.rs) is the decision, and it is
careful to be only that: [`due`](../../crates/guild/src/offer.rs) is a pure function of "how long
since the offer last looked," bounded by a day (chosen over `oh-my-zsh`'s thirteen because a guild
is worked on session to session, not fortnight to fortnight), so nothing here puts a network call
on a hot path — most invocations spend the whole check on one read of `machine.yml`.
[`eligible`](../../crates/guild/src/offer.rs) restates `armada_core::scan::handover`'s
both-stdin-and-stdout-are-a-terminal rule (Guild may not name Helm, so it is three booleans rather
than an import) plus the one guard `config scan` never needed: `ARMADA_JOB` unset, because a
Drone's exchange has nobody at the other end of stdin to answer at all. Nothing in that module ever
calls `fast_forward` — [`check`](../../crates/guild/src/offer.rs) only reports what the remote
looks like, `Outcome::Behind(_)` included, and every write stays on the helm side of the boundary.

The last-checked reading is a new field on Guild's own `machine.yml` section —
[`GuildSection::last_offer_ms`](../../crates/guild/src/machine.rs) — machine-local for the reason
`remote` and `withheld` already are (`PLAN.md` §13.1): a synced timestamp would let one machine's
check suppress another's offer, and they do not fetch from the same place at the same moment.
Recorded on every real attempt the check makes, offline included, so a stretch with no signal is
retried once a day rather than on every command typed during it — the same "offline is a warning,
never a failure" choice `doctor`'s `drift` already made, reused rather than re-argued.

[`crates/helm/src/verbs/offer.rs`](../../crates/helm/src/verbs/offer.rs) is the one place the
`yes` gets acted on, wired into `main.rs` right after `hand_over` — after the verb's own envelope
is written and flushed, so the question never competes with it for what the reader sees first. It
fires for every verb that reaches that point except the guild's own (asking *"pull now?"* right
after `armada guild pull` just ran would be noise about the thing the caller was already doing),
computed from the `Invocation` before `dispatch` moves it. The selector's default is "not now" —
`esc`, an unreadable line, and anything but a typed "pull" all take it — and only "pull" runs
`armada guild pull` itself, through a closure so the hard rule ("never pull without an explicit
yes") has a unit test that proves it without reproducing `pull`'s own git call sequence, which
already has its own tests in `crates/helm/src/verbs/guild.rs`.

**How a reader tells this apart from `armada guild upgrade`** (shipped the same day): `upgrade`
merges *Armada's own templates* into a guild, from the branch `docs/reserved/006` describes — it is
how a release reaches a guild that already exists. This is about *your* guild syncing between
*your* machines, and it is `armada guild pull` under the hood, offered rather than typed. Different
git history, different remote, different question entirely; they only look alike because both end
in a merge into `~/.armada/guild/`.
