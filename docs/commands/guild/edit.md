# `armada guild edit`

Open one guild file, validate it, and commit it.

> **Status: shipped.**

## Synopsis

```sh
armada guild edit <item> [--json]
armada guild edit <item> --from <file> [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `<item>` | name or guild-relative path | — | What to open — `voice.md`, `workflows/bug.yml`, `onboard-repo`. Required. |
| `--from <file>` | path | — | Replace its content from this file instead of opening a box. **The form that needs no terminal.** |

## How it works

1. Resolves `<item>`. A bare name works; a name two things answer to is refused with both
   named, because guessing is the one mistake a verb that writes may not make.
2. Opens it in the same inline text area the interview's prose questions use — `ctrl-d` saves,
   `esc` leaves it as it was. Under `--from`, the file is read instead and nothing is drawn.
3. **Validates it**, as the thing that consumes it reads it: a workflow through the parser
   Fleet uses, `settings.json` as JSON, `plugins.yml` and `mcp.yml` as YAML. Markdown is
   checked only for being there — nothing parses prose.
4. Commits, with a message naming what changed.

The commit is why this exists rather than telling you to open the directory yourself: an
uncommitted guild edit is the first half of the drift failure in
[`PHASES.md`](../../PHASES.md) §11. Committing means the only remaining step is a push.

## A refused edit is written and not committed

Losing somebody's work because a colon was in the wrong place is the worse of the two
failures, so the file is written either way. What does not happen is a workflow that no longer
parses reaching `push`: the history does not move, the refusal says why, and it names
`git -C ~/.armada/guild checkout <path>` as the undo.

## Output

```
  STATUS   ITEM               DETAIL
  REFUSED  workflows/bug.yml  `steps` is a required property

FAILED  ~/.armada/guild, not committed, git still holds the version before it
```

`--json` returns the item, the `outcome`, what the validator read, and `committed`.

## Dependencies

`git`, an initialised guild, and either a terminal or `--from`.

## Exit codes

`0` edited and committed · `1` `tool_failed` — validation failed, and **nothing was
committed** · `2` `bad_invocation` — no such item, an ambiguous name, or no terminal and no
`--from`.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`browse.md`](browse.md) · [`delete.md`](delete.md) · [`push.md`](push.md)
