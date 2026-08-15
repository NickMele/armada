# `armada guild show`

Print one guild file.

> **Status: shipped.**

## Synopsis

```sh
armada guild show <item> [--json]
```

## Arguments

| Argument | Type | Default | Meaning |
|---|---|---|---|
| `<item>` | string | required | A name or a guild-relative path — the names [`ls.md`](ls.md) prints. |

## Why this exists

[`ls.md`](ls.md) gives you nine rows and none of the files. `show` is the other half, and it is
the **non-interactive** half: an agent reading stdout, or a person through a pipe, gets a file's
content without a terminal being involved.

It is called `show` because that is the word [`manifest/skills.md`](../manifest/skills.md)
already uses for *one of them, in full* — one concept, one name across the modules
([`glossary.md`](../../glossary.md)).

## How it works

The whole file, indented and **unwrapped**. A viewer that reflowed a `SKILL.md` would show
something that is not what is on disk, and you are here precisely to see what is on disk; long
lines overhang instead.

**The terminal's `view` action runs this.** `guild ls` at a terminal builds the same envelope
and draws it through the same renderer, so what a person sees and what a pipe carries cannot
become two layouts maintained separately.

It writes nothing and takes no git action, so a name that resolves to two things is a refusal
rather than a guess — the same resolution [`edit.md`](edit.md) and [`delete.md`](delete.md) use.

## Output

```
  skills/onboard-repo/SKILL.md

  ---
  name: onboard-repo
  description: Write a repository's armada.yml with them.
  ---

  # Onboard a repository

  One question at a time, and nothing written before they confirm.

READY  ~/.armada/guild, skill, 6 KB
```

`--json` carries `body` — the file verbatim — alongside the same `item` row `ls` prints, so the
two answers cannot drift.

## Dependencies

An initialised guild.

## Exit codes

`0` shown · `2` `bad_invocation` — no guild on this machine, or nothing answers to that name.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`ls.md`](ls.md) · [`edit.md`](edit.md) · [`delete.md`](delete.md)
