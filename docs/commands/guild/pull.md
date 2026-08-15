# `armada guild pull`

Bring this machine's guild up to date.

> **Status: built — M2**, including re-projection ([`project.md`](project.md)).

## Synopsis

```sh
armada guild pull [--json]
```

## Arguments

Takes none beyond `--json`. Pulling is not a decision with options; the decisions are what to do
when it will not fast-forward, and those are reported rather than flagged.

## How it works

1. **Materialises the remote, if it is a folder.** iCloud Drive evicts the contents of files it
   thinks you are not using; a bare repository in that state is one git reports as corrupt for a
   repository that is intact and merely elsewhere. Armada asks for the evicted files back and
   waits up to 30s, then says so plainly rather than reporting damage.
2. Fetches.
3. **Fast-forwards if it can.** If it cannot, stops and reports the divergence with both commit
   counts — it never merges automatically, for the reason in [`push.md`](push.md).
4. **Re-projects** what arrived onto Claude Code's load path ([`project.md`](project.md)). A
   pulled guild that has not been projected is a guild that has not taken effect, and the gap
   between the two is a confusing hour. **Only on a fast-forward** — a divergence applied
   nothing, so there is nothing new to project and the step is skipped rather than run over a
   working tree you are about to resolve by hand.
   *(Plugin registrations and settings keys are the personal half [`PLAN.md`](../../PLAN.md)
   §13.3 says Guild writes itself, and are not projected by placing a file.)*

**A remote part-way through syncing is reported as a conflict, not a crash.** A push writes
several files and a sync service replicates them in its own order, so a machine reading in
between sees refs pointing at objects that have not arrived. That resolves itself; the row reads
`conflict  origin  part-way through syncing, nothing read` and the next action is to wait and
pull again. Nothing on this machine is touched. See [`PLAN.md`](../../PLAN.md) §13.5.

## Output

```
  STATUS     ITEM       DETAIL
  ADDED      skills     add-migration, triage-flake
  CHANGED    hooks      stop-notify.sh
  UNCHANGED  workflows  4

READY  pulled 4 commits, git@example.com:me/guild.git, projected 3 placed
```

**The projection is one fact on the summary line, not a second table.** What it did per file is
[`project.md`](project.md)'s whole output, and the verb is one word long.

**A file the projection left alone is named on the summary line and raises `NEEDS ATTENTION`**,
because a pull that says it worked while your own copy of a skill is the one still in effect is
worse than one that failed:

```
NEEDS ATTENTION  1 file left as yours in ~/.claude/, armada guild project shows which
```

`--json` returns the commit count, one result per changed area, and a `projected` object holding
`at`, one result per re-projected area, and `kept`.

## Dependencies

`git`, network, an initialised guild with a remote.

## Exit codes

`0` up to date · `1` `tool_failed` — diverged or the remote is mid-sync, and **nothing changed** · `2` `bad_invocation` — no remote configured · `4` `timeout` — a folder remote's files never finished downloading.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`push.md`](push.md) · [`project.md`](project.md) · [`../doctor.md`](../doctor.md)
