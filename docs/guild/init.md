# `armada guild init`

Build your guild by interviewing you.

> **Status: not built — M2.** ([`PHASES.md`](../PHASES.md) §8.4)

Usually reached through [`../init.md`](../init.md) rather than run directly. Run it directly to
rebuild a guild from scratch.

## Synopsis

```sh
armada guild init [--from <path>] [--no-import] [--remote <url>] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `--from <path>` | directory | `~/.claude` | Where to import an existing setup from. |
| `--no-import` | flag | off | Start empty. Skips step 1 entirely. |
| `--remote <url>` | git remote | — | Set the sync remote without being asked. |
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

### 2. Ask only what it cannot read

| Asked | Written to |
|---|---|
| How should agents write to you? | `voice.md` |
| What does "done" mean — coverage, review, commit style? | `expectations.md` |
| Branch conventions, when to ask vs decide, parallelism appetite | `how-i-work.md` |
| Confirm or edit the four starter workflows | `workflows/*.yml` |
| Default iteration, token and wall-clock ceilings | `workflows/*.yml` |

Each question is pre-filled from what the import found, so most are a confirmation rather than
an answer. Your existing memory file already answers most of the voice question.

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
