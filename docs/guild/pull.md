# `armada guild pull`

Bring this machine's guild up to date.

> **Status: not built — M2.**

## Synopsis

```sh
armada guild pull [--json]
```

## Arguments

Takes none beyond `--json`. Pulling is not a decision with options; the decisions are what to do
when it will not fast-forward, and those are reported rather than flagged.

## How it works

1. Fetches.
2. **Fast-forwards if it can.** If it cannot, stops and reports the divergence with both commit
   counts — it never merges automatically, for the reason in [`push.md`](push.md).
3. **Re-projects** what changed: re-registers plugins, re-writes managed memory regions,
   re-applies settings keys. A pulled guild that has not been projected is a guild that has not
   taken effect, and the gap between the two is a confusing hour.

## Output

```
pulled     4 commits from origin
projected  2 skills added · 1 hook changed · settings updated
```

`--json` returns the commit count and one result per re-projected item.

## Dependencies

`git`, network, an initialised guild with a remote.

## Exit codes

`0` up to date · `1` `tool_failed` — diverged, and **nothing changed** · `2` `bad_invocation` — no remote configured.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`push.md`](push.md) · [`../doctor.md`](../doctor.md)
