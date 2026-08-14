# `armada guild edit`

Open the guild for editing, and commit what changes.

> **Status: not built — M2.**

## Synopsis

```sh
armada guild edit [<file>] [--no-commit] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `<file>` | guild-relative path | guild root | What to open — `voice.md`, `workflows/bug.yml`, and so on. |
| `--no-commit` | flag | off | Do not commit afterwards. For a series of edits you want in one commit. |

## How it works

1. Opens `<file>` (or the guild directory) in `$EDITOR`.
2. On exit, validates anything schema-backed — workflow files, `plugins.yml`, `mcp.yml`.
   **Invalid content is not committed**, and the error names the key path.
3. Commits with a generated message describing what changed.

The commit is why this exists rather than telling you to open the directory yourself: an
uncommitted guild edit is the first half of the drift failure in [`PHASES.md`](../PHASES.md) §11.
Committing automatically means the only remaining step is a push.

## Output

```
edited    workflows/bug.yml
valid     ✓
commit    guild: raise bug workflow iteration ceiling to 16
```

`--json` returns the changed paths, the validation result, and the commit id.

## Dependencies

`$EDITOR`, `git`, and an initialised guild.

## Exit codes

`0` edited and committed · `2` `bad_invocation` — no such file in the guild · `3` `bad_config` — validation failed, and **nothing was committed**.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`init.md`](init.md) · [`push.md`](push.md)
