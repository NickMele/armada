# `armada guild init`

Build your guild by interviewing you.

> **Status: not built — M2.** ([`PHASES.md`](../../PHASES.md) §8.4)

Usually reached through [`../init.md`](../init.md) rather than run directly. Run it directly to
rebuild a guild from scratch.

## Synopsis

```sh
armada guild init [--from <path>] [--no-import] [--remote <url>] [--defaults] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `--from <path>` | directory | `~/.claude` | Where to import an existing setup from. |
| `--no-import` | flag | off | Start empty. Skips step 1 entirely. |
| `--remote <url>` | git remote | — | Set the sync remote without being asked. |
| `--defaults` | flag | off | Take every default answer. Leaves a working guild, reported incomplete by `armada doctor`. |
| `--force` | flag | off | Overwrite an existing guild. Refuses without it. |

## How it works

### 1. Import what already exists

Reads `~/.claude/` and adopts it: skills, subagents, hooks, plugin and marketplace
registrations, settings, and the memory file. **The guild starts nearly complete rather than
empty** — which is the difference between a tool you configure once and one you abandon during
configuration.

The importer **refuses to adopt credential-shaped values.** Anything that looks like a token
goes to `machine.yml`, which never syncs. A secret that has reached a remote cannot be
un-pushed, so this is built here rather than retrofitted.

### 2. Ask five questions, from scratch

| # | Asked | Written to | Default if skipped |
|---|---|---|---|
| 1 | How should agents write to you? | `voice.md` | what import wrote |
| 2 | What does "done" mean — coverage, review, commit style? | `expectations.md` | what import wrote |
| 3 | How do you work? Branching, when to ask versus decide, parallelism appetite. | `how-i-work.md` | what import wrote |
| 4 | Default iteration, token and wall-clock ceilings | `workflows/*.yml` | the per-workflow ceilings of [`PLAN.md`](../../PLAN.md) §14.6 |
| 5 | A private git remote to sync to | `machine.yml` | none — sync off, `export` still works |

**Questions are not pre-filled with the import's guess.** Reviewing a machine's interpretation
of your own memory file is more work than answering, and it produces a worse answer: you end up
editing its reading rather than saying what you mean. Import populates the files; these
questions ask fresh; your answers win where they overlap.

**The four starter workflows are copied, not confirmed.** A confirmation step on a file you
have not read yet is theatre — [`edit.md`](edit.md) changes them once you have an opinion.

`--defaults` takes every default and finishes. The guild works; `armada doctor` reports which
fragments are still whatever import produced.

### 3. Initialise the repository

`~/.armada/guild/` becomes a git repository with an initial commit. If `--remote` was given or
answered, it is set and pushed.

**Everything the interview writes is a plain file you can edit afterwards.** The interview is a
convenience and never the only way in — a tool that can only be configured through a wizard is
a tool you cannot fix at one in the morning.

## Output

```
imported  19 skills · 12 hooks · 4 plugins · 2 marketplaces · settings · memory
withheld  1 credential-shaped value → machine.yml
wrote     voice.md · expectations.md · how-i-work.md · 4 workflows
guild     initialised, remote <url>
```

`--json` returns one result per imported category and one per written file.

## Dependencies

| On | Why |
|---|---|
| `git` | The guild is a git repository. |
| `~/.claude/` | Only for the import step; absent is fine, it starts empty. |
| A terminal | The interview is interactive. Use `--no-import` plus [`edit.md`](edit.md) for scripted setup. |

## Exit codes

`0` guild ready · `1` `tool_failed` — the import found something it could not parse · `2` `bad_invocation` — a guild exists and `--force` was not given.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`../init.md`](../init.md) · [`edit.md`](edit.md) · [`push.md`](push.md)
