# `armada guild push`

Send local guild changes to the remote.

> **Status: built — M2.**

## Synopsis

```sh
armada guild push [--force] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `--force` | flag | off | Force-push. Refused unless the remote is strictly behind — this never silently discards another machine's commits. |

## How it works

1. Commits anything uncommitted, so an edit made outside `edit.md` is not left behind.
2. Fetches and compares. **Diverged histories stop here** with the two commit counts and
   instructions, rather than being merged automatically. An automatic merge of two machines'
   guilds is how you end up with a hook you did not write.
3. Pushes.

## Output

```
pushed  3 commits → origin
```

Or, on divergence:

```
diverged  local 2 ahead, 1 behind — run `armada guild pull` first
```

`--json` returns the commit count, remote, and resulting head.

## Dependencies

`git`, network, an initialised guild with a remote. Without a remote it exits `2` and points at
[`export.md`](export.md).

## Exit codes

`0` pushed, or already up to date · `1` `tool_failed` — the remote rejected it, or the histories have diverged · `2` `bad_invocation` — no remote configured; see [`export.md`](export.md).

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`pull.md`](pull.md) · [`export.md`](export.md)
