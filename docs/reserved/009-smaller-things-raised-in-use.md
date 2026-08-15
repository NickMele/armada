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
| 1 | `doctor` says `manifest.db` is present and nothing else | Presence is not health. Row counts per table, and how much of it is stale — a store that exists and holds four thousand dead rows is a different answer from one that exists |
| 2 | `config scan` offers the agent hand-over or nothing | Three options: build it with an agent, **write a blank `armada.yml` and open it in `$EDITOR`**, or stop. The middle one is missing and is what a reader who knows their own repository wants |
| 3 | The guild has a remote and nothing reports on it | Whether there is anything to pull, and anything local worth pushing. Same shape as `git status` and it is the same question |
| 4 | Sync is manual and easy to forget | An occasional offer to pull when the guild is behind — `oh-my-zsh`'s prompt is the reference. **Never automatic**: a `pull` that runs unasked can change how an agent behaves mid-session |
| 5 | `armada --help`'s `NOT BUILT YET` list has drifted from this plan | It lists `guild edit, verify`, `manifest render, agents-md, explain`, `check --detach/--status` — and nothing reserved in `docs/reserved/`. Reconciling them is a chore with a test behind it, and the test is what stops it drifting again |

**Two of these are the same rule.** #1 and #5 are both a report that has stopped tracking what
it describes. `doctor` describes a file it does not look inside; `--help` describes a plan it
does not read. Wherever a listing is derived rather than retyped — `args.rs`'s own constants
already feed the help page — it cannot drift, and that is the fix in both cases.
