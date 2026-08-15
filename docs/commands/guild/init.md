# `armada guild init`

Build your guild by interviewing you.

> **Status: built — M2.** ([`PHASES.md`](../../PHASES.md) §8.4)

Usually reached through [`../init.md`](../init.md) rather than run directly. Run it directly to
rebuild a guild from scratch.

## Synopsis

```sh
armada guild init [--from <path>] [--no-import] [--remote <url>] [--defaults] [--force]
                  [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `--from <path>` | directory | `~/.claude` | Where to import an existing setup from. |
| `--no-import` | flag | off | Start empty. Skips step 1 entirely. |
| `--remote <url\|path>` | git remote or folder | — | Set the sync remote without being asked. A folder is made a bare repository and used as one. |
| `--defaults` | flag | off | Take every default answer. Leaves a working guild, reported incomplete by `armada doctor`. |
| `--force` | flag | off | Overwrite an existing guild. Refuses without it. |

## How it works

### 1. Import what already exists

Reads `~/.claude/` and adopts it: skills, subagents, hooks, plugin and marketplace
registrations, settings, and the memory file. **The guild starts nearly complete rather than
empty** — which is the difference between a tool you configure once and one you abandon during
configuration.

The importer **refuses to adopt credential-shaped values.** Anything that looks like a token
goes to the `guild:` section of `machine.yml`, which never syncs — **by key, never by value**
([`PLAN.md`](../../PLAN.md) §13.1). A secret that has reached a remote cannot be un-pushed, so
this is built here rather than retrofitted.

`CLAUDE.md` is carved into `voice.md`, `expectations.md` and `how-i-work.md`. **Each opens with
what it is for, not with where it came from** — its heading, who reads it and when, and what to
write in it — and where import found nothing, Armada writes four examples under a heading that
says they are examples. A file you have to invent from a blank page is a file you close again.
Every fragment Armada writes carries a marker, and `armada doctor` names it until you replace
it: `still as imported`, or `still Armada's example text`. See
[`PLAN.md`](../../PLAN.md) §13.4.

### 2. Ask five questions, from scratch

| # | Asked | Written to | Default if you press enter |
|---|---|---|---|
| 1 | How should agents write to you? | `voice.md` | what import wrote |
| 2 | When is work actually finished? | `expectations.md` | what import wrote |
| 3 | How should agents work in your repos? | `how-i-work.md` | what import wrote |
| 4 | How much should one Job spend before it stops and asks you? | `workflows/*.yml` | `20, 600k, 90m` |
| 5 | Where should your guild sync to? | `machine.yml` | none — sync off, `export` still works |

**Every question says what answer it wants, and puts the current one in front of you.**

```
2/5  When is work actually finished?
     What must be true before an agent tells you it is done: tests passing, a
     review, a branch, a changelog entry. Workflows gate on this. Writes
     expectations.md.
     enter for a new line · ctrl-d saves · esc keeps it as it was
┌──────────────────────────────────────────────────────────────────────┐
│ Tests pass, lint is clean, and someone has read the diff.            │
│                                                                      │
│ New behaviour has a test that fails without it.▌                     │
└─────────────────────────────────────────────── 12 more below ────────┘
```

The prompt is the question. The paragraph under it says what shape of answer is wanted and which
file it lands in — *"What does "done" mean — coverage, review, commit style?"* did not. And the
block ends with a blank line, because without one the last answer runs straight into the summary
table and the interview and its result read as one thing.

**Questions 1–3 open a real editor**, inline, with wrapping and arrow keys — they want
paragraphs, and a single-line prompt that scrolls sideways is not a place anyone writes one.
Questions 4 and 5 are one short structured value each and stay a single line, with a `now` line
above them showing the default. Neither opens `$EDITOR`; you never leave the interview.

### The box opens holding what you already have

| Key | Does |
|---|---|
| `ctrl-d` | saves what is in the box, edited or not |
| `esc`, `ctrl-c` | keeps it as it was — the file is not touched |

Those two are different answers. `ctrl-d` on a box you did not change means *I have read this and
it is mine*, and [`../doctor.md`](../doctor.md) stops naming the file; `esc` means *leave it*, and
it does not. The box **scrolls**, and its bottom border says how many lines are still below —
an imported fragment is easily thirty.

**This reverses an earlier rule, and the reversal is the point.** The interview used to quote the
current value on a `now` line and ask fresh, on the argument that editing a machine's reading of
your memory file is more work than answering. What that produced was a line cut off at the width
of the terminal — and, in the text area, drawn a second time without accounting for wrapping, so
it ran off the edge. A default you cannot read is not one you can accept, and retyping a
paragraph from a truncated echo is not less work than editing it.

What survives is the half that was actually load-bearing: **you are never asked to confirm the
split.** Armada does not show you which sections it filed where and ask you to correct it. It
shows you one file, whole, and lets you write in it.

**The four starter workflows are copied, not confirmed.** A confirmation step on a file you
have not read yet is theatre — [`edit.md`](edit.md) changes them once you have an opinion.

`--defaults` takes every default and finishes. The guild works; `armada doctor` reports which
fragments are still whatever import produced.

### 3. Initialise the repository

`~/.armada/guild/` becomes a git repository with an initial commit. If `--remote` was given or
answered, it is set and pushed.

**A folder is a remote.** Question 5 takes a git URL or a filesystem path — iCloud Drive, a NAS
mount, a drive you plug in. Given a path, Armada creates it if needed, runs `git init --bare` in
it, and uses it as the remote; git speaks a filesystem remote natively, so everything after that
is ordinary. A folder that is already a repository is adopted rather than re-initialised, which
is what makes the second machine naming the same folder work. Two caveats that come with a sync
service rather than with git — eviction and a half-replicated push — are handled and explained in
[`PLAN.md`](../../PLAN.md) §13.5.

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

**A `migrated` row appears once, on a machine whose `machine.yml` predates the sections.**
That file carries one top-level section per module ([`PLAN.md`](../../PLAN.md) §4.3.1); a file
written before they existed has one module's keys loose at the top level, and this verb is
already writing to it, so it moves them and says which:

```
  migrated  machine.yml  cpu_slots, port_block_size moved under `manifest:`
```

It is drawn only when the migration happened — it happens once per machine, and a row that
appeared every run is the row nobody reads. Armada never rewrites this file on a *read*: a
verb that only inspects your configuration leaves a hand-edited file exactly as it found it.

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
