# `armada guild pull`

Bring this machine's guild up to date.

> **Status: built — M2**, less the re-projection step, which needs a projector
> and lands with one. A pull today updates the guild; re-writing what Claude
> Code reads from it is the projector's half.

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
4. **Re-projects** what changed: re-registers plugins, re-writes managed memory regions,
   re-applies settings keys. A pulled guild that has not been projected is a guild that has not
   taken effect, and the gap between the two is a confusing hour.

**A remote part-way through syncing is reported as a conflict, not a crash.** A push writes
several files and a sync service replicates them in its own order, so a machine reading in
between sees refs pointing at objects that have not arrived. That resolves itself; the row reads
`conflict  origin  part-way through syncing, nothing read` and the next action is to wait and
pull again. Nothing on this machine is touched. See [`PLAN.md`](../../PLAN.md) §13.5.

## Output

```
pulled     4 commits from origin
projected  2 skills added · 1 hook changed · settings updated
```

`--json` returns the commit count and one result per re-projected item.

## Dependencies

`git`, network, an initialised guild with a remote.

## Exit codes

`0` up to date · `1` `tool_failed` — diverged or the remote is mid-sync, and **nothing changed** · `2` `bad_invocation` — no remote configured · `4` `timeout` — a folder remote's files never finished downloading.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`push.md`](push.md) · [`../doctor.md`](../doctor.md)
