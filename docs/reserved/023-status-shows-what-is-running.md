---
id: 023
title: Status shows what is running
status: FIXED
module: manifest
raised: real use, 2026-08-15
---

# 023 — Status shows what is running

> **Fixed.** `armada manifest status` enumerates every workspace the store holds *any* state
> for rather than only the ones that claimed a port block; it names the subset of what a
> workspace owns that is provably gone, in `stale[]`; and it reports detached `check` runs that
> have not reached a verdict, as `RUNNING` or `DEAD`. What follows is the measurement and the
> argument, kept because the alternative fix is the one a later change is most likely to reach
> for.

## The defect, measured

`armada manifest status` opens its own documentation with *"what is running, what is mine, what
is stale"* ([`commands/manifest/status.md`](../commands/manifest/status.md)). On 2026-08-15 it
printed nothing at all, on this repository, while a detached `check` run was executing.

| Table in `~/.armada/manifest.db` | Contents |
|---|---|
| `workspaces` | **0 rows** |
| `owned` | **6 rows**, all `workspace = c24a68b6`, all `kind = pgid` |

`c24a68b6` is `sha1(realpath)` of this project's own checkout. Four of the six carried boot id
`406FE4B6-…` — a **previous boot**, so they were leaked by definition; the other two carried the
boot the machine was on. `armada manifest status --all --json` answered `"results": []` and exit
`0`.

Each of the six `owned` rows pairs one-to-one with a directory under `.armada/run/`, so all six
were `check --detach` runs and none of them was a service.

## Why `workspaces` was empty while `owned` was not

**The two tables have different preconditions, and both writers are correct.**

| | `workspaces` | `owned` |
|---|---|---|
| written by | `Db::claim_block`, one call site: `armada manifest init` | `up`, `check --detach`, `fleet spawn` |
| records | a **port claim** | a **resource that exists** |
| needs | the verb to have been run | nothing — the key is `WorkspaceId::derive`, `sha1(realpath)` |

This repository declares no `ports:`, so nobody ever had a reason to run `armada manifest init`
in it — `~/.armada/recent.jsonl` records `fleet tick`, `manifest status`, `manifest check`, `mcp
serve`, `helm` and `helm enable`, and no `init` at all. Meanwhile every `armada manifest check
--detach` recorded a `pgid` against the derived id. The store was in exactly the state its design
permits.

`status` then enumerated `Db::workspaces()` and asked each of its **zero** rows what it owned. So
did `clean`'s pass 1, and so does `Db::peek_stats` — see *what this does not fix*, below.

## The fix, and the one that was refused

Two fixes were available.

| | Fix | Verdict |
|---|---|---|
| **a** | `status` enumerates the union of `workspaces`, `owned` and `leases` | **taken** |
| **b** | nothing may write an `owned` row for a workspace absent from `workspaces` | **refused** |

**(b) is refused on three counts, and the first is fatal on its own.** `check --detach` would have
to either refuse to run in a repository that never claimed ports — which would break M4's loop and
this repository's own dogfooding, since `check` needs no port block and has never required
`init` — or claim one as a side effect, which would make a CI verb take a machine-global
reservation nobody asked for. Second, a rule that only forbids *future* writes cannot see the six
rows already on the machine, so the measured leak would stay invisible forever. Third, it
mistakes which table is authoritative: a process group exists whether or not anything wrote down
that it might. `owned` is the record of what is real; `workspaces` is the record of what was
claimed.

So the store answers the union itself, in `Db::known_workspaces` — one statement, one place —
rather than each caller assembling it, because a second hand-rolled union is how two verbs come
to disagree about what exists.

**The workspace you are standing in always gets a row**, even when the store holds nothing for
it. `owns  resources  —` means Armada looked and found nothing; no row at all means nothing
whatsoever, and a reader cannot tell those apart — the argument the renderer already makes for
keeping an empty resource row.

## What "stale" means, and why it is a field rather than a status word

`status.md` has promised three states per resource — **live**, **stale**, **unowned** — since it
was written, and the payload could express exactly one of them. Four dead process groups and two
live ones printed identically.

`stale[]` is the subset of `owns[]` that Armada can **prove** is gone, under
`reap::pgid_is_ours` — the same rule `clean` kills on, so nothing lands there that `clean` would
decline to reclaim. That makes it an instruction rather than a worry. A container, network,
volume or image is never judged: deciding would take a daemon call, and `status` asks no daemon,
which is what makes it cheap enough to poll.

**`STALE` is not a new `Status`.** [`glossary.md`](../glossary.md) fixes the Manifest status enum
at fourteen words and `STALE` is not among them; adding a fifteenth for a *resource* would put a
word in an enum whose members are the outcomes of a *run*. It is a payload field and a render
token, in the same lowercase family as `OWNS`, `HELD`, `COLD` and `REPORTED` — the words that
table already uses — and it is the word `status.md` itself chose.

## What it now reports about a run

The design ask, in the repository owner's words:

> *"Not only should the bridge show it, but I would think that `arm manifest status` should show
> running checks and anything that is 'up'."*

A detached run returns immediately, and the only way to see it was `check --status` with the run
id in hand — a question only somebody who already had the answer could ask. `check`'s own source
said so, in the remedy it offers for an unknown run id: *"not 'list them', because nothing lists
them."*

`status` now reports, per workspace, **the detached runs that have not reached a verdict, from
this boot**, as `RUNNING` or `DEAD` — the same two words `check --status` reaches, by the same two
questions in the same order: the record holds no verdict, and the recorded group is provably
still this run.

Three exclusions, each for a reason rather than for brevity:

1. **A run that reached a verdict is history.** `runs[]` answers *what is running*; a workspace
   keeping a retention count of decided runs would otherwise report the same finished run on
   every poll forever, and `check --status <id>` is the verb that reads a verdict back.
2. **A run from a previous boot cannot be executing**, and whatever it leaked is already named in
   `stale[]` as the `pgid` `clean` will drop. Excluding it also bounds the reads: `detached.json`
   is five fields, `state.json` carries the whole journal, and parsing every run a workspace ever
   kept on every poll is not what a pollable verb does.
3. **A foreground run recorded no group to ask about**, and its caller is waiting on it.

Every filesystem failure under `.armada/run/` answers *no runs* rather than failing the verb.
`status` exits `0` whenever **the store** could be read, and a run directory is not the store — a
directory being reaped by a concurrent `check` as this reads it must not turn a machine-wide
query into an `environment` failure.

## What this does not fix

**`clean` and `doctor` are blind in exactly the same way, and were left alone.** `App::plan_reap`
pass 1 enumerates `Db::workspaces()` and calls `stop_owned_processes` only for the ids that pass
names, so an `owned` row whose workspace has no registry row is unreachable to the reaper as
well; `Db::peek_stats` computes `reclaimable_owned` by intersecting `owned` against the same
registry-derived set, so `armada doctor` counted the six leaked rows as zero. Both now have
`Db::known_workspaces` available. `clean`'s case needs a decision this note does not take: a
workspace with no registry row has **no recorded path**, and the reap rule is *"the workspace's
directory is gone"* — which is a question that cannot be asked of a one-way hash.

**`check --detach` never deletes the `owned` row it records.** The pgid row is written after the
spawn and removed by nothing; only `clean` reclaims it. That is why six runs left six rows. It is
a defect in `check` rather than in `status`, and `status` reporting them as `stale[]` is what
makes it visible rather than what fixes it.
