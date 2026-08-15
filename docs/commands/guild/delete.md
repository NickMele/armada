# `armada guild delete`

Remove one thing from your guild, and commit the removal.

> **Status: shipped.**

## Synopsis

```sh
armada guild delete <item> [--yes] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `<item>` | name or guild-relative path | — | What to remove. Required. |
| `--yes` | flag | off | Skip the confirmation. **Required where there is nobody to ask.** |

## How it works

1. Resolves `<item>`, refusing an ambiguous name with both candidates named.
2. Reads the rest of the guild for anything that still names it, and puts that in the
   confirmation. Keeping it is the default, so `esc` and an ended stream both leave it alone.
3. Removes it — the whole directory for a skill, the file for everything else.
4. **Commits the removal.**

## Why the commit is not optional

The guild is a git worktree that syncs between machines ([`PLAN.md`](../../PLAN.md) §13.1). A
delete that only unlinked a file would leave this machine's working tree ahead of its history,
`armada guild push` would carry nothing, and the two machines would disagree about a file one
of them believes is gone. Committing is also what makes it recoverable: `git -C
~/.armada/guild revert` puts it back.

## What the reference check covers, and what it does not

**It reads the guild.** A workflow step naming a skill, a workflow naming a sub-workflow, a
subagent naming either — those are text in files this can read, and they are reported.

**It does not read your repositories.** A project's `armada.yml` naming a workflow is not
checked and cannot be from here: those live in repositories Guild has no register of, and Guild
may not ask Manifest for one ([`ARCHITECTURE.md`](../../ARCHITECTURE.md) §1.9). The row says
what was checked rather than implying it checked everything.

## Output

```
  STATUS      ITEM                  DETAIL
  DELETED     skills/add-migration  Write a migration and its rollback.
  REFERENCED  1 file                workflows/feature.yml

READY  ~/.armada/guild, committed, armada guild push sends it
```

`--json` returns the item, the `outcome`, `committed`, and `referenced_by`.

## Dependencies

`git`, an initialised guild, and either a terminal or `--yes`.

## Exit codes

`0` removed and committed, or kept · `2` `bad_invocation` — no such item, an ambiguous name, or
no terminal and no `--yes`.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`browse.md`](browse.md) · [`edit.md`](edit.md) · [`push.md`](push.md)
