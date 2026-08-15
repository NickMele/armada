# `armada guild ls`

See what is in your guild.

> **Status: shipped.**

## Synopsis

```sh
armada guild ls [--list] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `--list` | flag | off | Print the listing and stop, rather than navigating it. |

There is deliberately **no flag that turns the navigating on**. A terminal is the flag
([`PLAN.md`](../../PLAN.md) §3.1.1); `--list` only goes the other way, so the interactive and
printed forms can be compared from one terminal.

## Why this exists

Every other `guild` verb **moves** the guild — onto this machine, into a repo, into a bundle,
to the remote — and not one of them said what was in it. `export` writing a bundle was the
standing answer to *what do I have*, and it is not an answer
([`PLAN.md`](../../PLAN.md) §15.3.4).

## Why it is called `ls`

Because [`fleet/ls.md`](../fleet/ls.md) is already the word for a listing, and
[`manifest/skills.md`](../manifest/skills.md)'s `show` is already the word for one of them in
full. One concept, one name, across the four modules — that is what
[`glossary.md`](../../glossary.md) is for, and a third spelling of *listing* in a third module
is exactly the drift it was written to stop.

## How it works

One row per thing, in the universal shape — `STATUS · ITEM · DETAIL` — where **STATUS is the
kind**: `MEMORY`, `SKILL`, `SUBAGENT`, `WORKFLOW`, `HOOK`, `SETTINGS`, `PLUGINS`, `MCP`,
`SCHEMA`. Each kind is summarised in its own terms, read out of the file rather than guessed
from its name: a workflow by its steps, a skill by its front-matter `description`, a memory
fragment by whether it is still Armada's example text.

**Whether a fragment is still Armada's words is read out of the file every time**, from a
marker the text carries — not from when the file was last written. Edit a fragment and the row
changes; put the example back and the row changes back.

**The listing is the verb; navigating it is one way of reading it.** At a terminal, the rows
become a selector: pick one, then `view`, `edit` or `delete` it. Through a pipe, or under
`--list`, the same rows are printed. `--json` carries them again. An interactive-only verb
would be a bug ([`PLAN.md`](../../PLAN.md) §3.1.1), so all three audiences see identical facts.

**`view` runs [`show.md`](show.md).** The action is spelled *view* on the screen because that
is what a person picking off a list is doing, but it builds the same envelope `guild show`
returns and draws it through the same renderer — one layout, not two that agree today.

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

At a terminal the same rows are drawn as a selector by
[`ask/select.rs`](../../../crates/helm/src/ask/select.rs) — the one selector in the binary —
titled *What is in your guild?*, with `done` last and default. Picking a row asks a second
question titled with the item's path, offering `view`, `edit`, `delete` and `back`.

`--json` returns each item's `kind`, `name`, `path`, `opens`, `detail` and `bytes`. `path` is
what a delete removes and `opens` is what a `show` or an edit reads — for a skill those differ.

## Dependencies

An initialised guild.

## Exit codes

`0` listed · `2` `bad_invocation` — there is no guild on this machine.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`show.md`](show.md) · [`edit.md`](edit.md) · [`delete.md`](delete.md) · [`init.md`](init.md)
