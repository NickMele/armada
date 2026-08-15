# `armada guild init`

Build your guild by interviewing you.

> **Status: built — M2.** ([`PHASES.md`](../../PHASES.md) §8.4)

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

| # | Asked | Written to | Default if you press enter |
|---|---|---|---|
| 1 | How should agents write to you? | `voice.md` | what import wrote |
| 2 | When is work actually finished? | `expectations.md` | what import wrote |
| 3 | How should agents work in your repos? | `how-i-work.md` | what import wrote |
| 4 | How much should one Job spend before it stops and asks you? | `workflows/*.yml` | `20, 600k, 90m` |
| 5 | Where should your guild sync to? | `machine.yml` | none — sync off, `export` still works |

**Every question says what answer it wants, and shows what enter would keep.**

```
2/5  When is work actually finished?
     What must be true before an agent tells you it is done: tests passing, a
     review, a branch, a changelog entry. Workflows gate on this. Writes
     expectations.md.

     now  Tests pass, lint is clean, and someone has read the diff.
     enter keeps what import found  >
```

Four things are in that block and each is there because the first real run of this interview
did without it. The prompt is the question. The paragraph under it says what shape of answer is
wanted and which file it lands in — *"What does "done" mean — coverage, review, commit style?"*
did not. The `now` line is what pressing enter would keep, because a default you cannot see is
not one you can accept with confidence. And there is a blank line above every question, because
five of them with nothing in between ran together.

**Questions 1–3 open a real editor**, inline, with wrapping and arrow keys — they want
paragraphs, and a single-line prompt that scrolls sideways is not a place anyone writes one.
Questions 4 and 5 are one short structured value each and stay a single line. Neither opens
`$EDITOR`; you never leave the interview.

**Questions are not pre-filled with the import's guess.** The `now` line *shows* what import
found; it does not put it in the box for you to edit. Reviewing a machine's interpretation of
your own memory file is more work than answering, and it produces a worse answer: you end up
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
  STATUS    STEP         DETAIL
  imported  ~/.claude/   19 skills, 12 hooks, 4 plugins
  withheld  1 value      settings.json:env.GITHUB_TOKEN -> machine.yml, which never syncs
  wrote     4 files      voice.md, expectations.md, how-i-work.md, +1
  guild     initialised  git@example.com:me/guild.git

READY  guild at ~/.armada/guild, 1 answered, 4 kept as imported
```

**The `withheld` row appears only when something was withheld.** It used to be drawn either way,
on the argument that "Armada looked and found nothing" and "nobody looked" are different facts.
What it printed was `withheld  0 values  no credential-shaped values found`, which says nothing
three times and names neither what was checked nor against what — a row a reader learns to skip,
and then skips on the day it says `1 value`. The guarantee is unchanged and stated here; the
line claiming to report it is gone.

**`kept as imported`, not `skipped`.** Pressing enter is what the hint instructs and it accepts a
value. Calling that skipping told someone who had followed the instructions that he had done
nothing.

`--json` returns one result per imported category and one per written file, plus `answered` — how
many of the five you typed an answer to.

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
