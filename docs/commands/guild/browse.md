# `armada guild browse`

See what is in your guild, and open it.

> **Status: shipped.**

## Synopsis

```sh
armada guild browse [--list] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `--list` | flag | off | Print the listing and stop, rather than opening the browser. |

## Why this exists

Every other `guild` verb **moves** the guild — onto this machine, into a repo, into a bundle,
to the remote — and not one of them said what was in it. `export` writing a bundle was the
standing answer to *what do I have*, and it is not an answer
([`PLAN.md`](../../PLAN.md) §15.3.4).

## How it works

One row per thing, in the universal shape — `STATUS · ITEM · DETAIL` — where **STATUS is the
kind**: `MEMORY`, `SKILL`, `SUBAGENT`, `WORKFLOW`, `HOOK`, `SETTINGS`, `PLUGINS`, `MCP`,
`SCHEMA`. Each kind is summarised in its own terms, read out of the file rather than guessed
from its name: a workflow by its steps, a skill by its front-matter `description`, a memory
fragment by whether it is still Armada's example text.

**The listing is the verb; the browser is one way of reading it.** At a terminal, the rows
become a selector: pick one, then `view`, `edit` or `delete` it. Through a pipe, or under
`--list`, the same rows are printed. `--json` carries them again. An interactive-only verb
would be a bug ([`PLAN.md`](../../PLAN.md) §3.1.1), so all three audiences see identical facts.

**It says what is there, not what has moved.** [`pull.md`](pull.md) already reports drift and
[`doctor.md`](../doctor.md) already reports what is wrong; this is the prior question, and it
is the one you need first when a guild has drifted between machines.

## Output

```
  STATUS    ITEM                  DETAIL
  MEMORY    voice.md              150 words maximum. Lead with the answer.
  MEMORY    expectations.md       still Armada's example text
  SKILL     onboard-repo          Write a repository's armada.yml with them.
  SUBAGENT  helm.md               The one agent you talk to.
  WORKFLOW  bug.yml               4 steps, reproduce, fix, review, land
  HOOK      stop-notify.sh        sh, 12 lines
  SCHEMA    workflow.schema.json  what every workflow is checked against

READY  ~/.armada/guild, 1 skill, 1 hook, 1 subagent, 1 workflow
```

`--json` returns each item's `kind`, `name`, `path`, `opens`, `detail` and `bytes`. `path` is
what a delete removes and `opens` is what a view or an edit reads — for a skill those differ.

## Dependencies

An initialised guild.

## Exit codes

`0` listed · `2` `bad_invocation` — there is no guild on this machine.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`edit.md`](edit.md) · [`delete.md`](delete.md) · [`init.md`](init.md)
