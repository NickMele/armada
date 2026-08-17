# `armada init`

Set up **this machine**. Run once per box.

> **Status: built — M2.** ([`PHASES.md`](../PHASES.md) §8.4)

Not to be confused with [`manifest/init.md`](manifest/init.md), which sets up a *workspace*.
This sets up *you, here*.

## Synopsis

```sh
armada init [--guild <remote>] [--defaults] [--force] [--json]
armada init --bundle <path> [--defaults] [--force] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `--guild <remote>` | git remote URL | — | Skip the prompt and pull an existing guild from this remote. |
| `--bundle <path>` | file path | — | Skip the prompt and import an existing guild from a bundle. |
| `--defaults` | flag | off | Take the default answer to every interview question. Leaves a **working** guild, not an empty one, and `armada doctor` reports it as incomplete. |
| `--force` | flag | off | Re-run against a machine that has already been set up, recreating whatever is missing. Refuses without this. |

`--guild` and `--bundle` are mutually exclusive.

**"Already set up" is the layout, not the directory.** The refusal asks whether `guild/`, `jobs/`
or `workspaces/` is there — not whether `~/.armada/` is. Armada writes `recent.jsonl`,
`untried.jsonl` and `failures.jsonl` into that directory from the first command you ever run, so
one `armada doctor` on a new machine creates it; reading that as *you have already run `init`*
made this verb refuse the first run on every machine, and told the caller their command was
wrong for a directory Armada had made itself.

**`--force` never replaces a guild.** A re-run puts back any missing directory and, if a guild is
already here, does not ask the question and does not write a byte of it — which is what makes
`armada init --force` the repair [`doctor.md`](doctor.md) names for a missing `jobs/`. Replacing
a guild is [`guild/init.md`](guild/init.md) `--force`, which says so in the words you typed.

## How it works

1. **Preflight.** Checks for `git`, `claude`, and a container runtime. Reports each as present
   or missing with the version found. Missing `claude` is fatal; a missing container runtime is
   a warning, because not every repo needs one.
2. **Creates `~/.armada/`** — `guild/`, `jobs/`, `workspaces/`, `manifest.db`,
   `machine.yml`.
3. **Asks the one question that matters:** *do you already have a guild?*
   - **Pull from a remote** — clones it into `~/.armada/guild/`. Done in seconds; this is the
     second-machine path.
   - **Import a bundle** — unpacks a file. For a machine that will never hold your credentials.
   - **Build one now** — imports `~/.claude/` first, then asks **seven** questions
     ([`PLAN.md`](../PLAN.md) §13.4). Import does most of the work; the interview asks only
     what cannot be read from the machine, and every question has a default.
4. **Writes `machine.yml`** — paths, capacity, and anything machine-specific, under **one
   top-level section per module** ([`PLAN.md`](../PLAN.md) §4.3.1). This file **never syncs**
   ([`PLAN.md`](../PLAN.md) §13.1). A file written before the sections existed is moved into
   them here, and the run says which keys moved.
5. **Projects the guild** onto Claude Code's load path
   ([`guild/project.md`](guild/project.md)) — **whichever of the three answers brought it
   here**. A guild in `~/.armada/guild/` that nothing has projected is one Claude Code cannot
   see, and this verb's promise is a working setup rather than a directory. Any file you had
   already written is left exactly as it is and reported; nothing here overwrites your own work.

## Output

A transcript: the checklist, the one question and what was typed, what import adopted, each
interview prompt as it was put, and the verdict. Frozen byte for byte by
`tests/golden/render/init-machine.plain`.

```
  STATUS   CHECK      DETAIL
  found    git        2.51.0
  found    claude     2.0.14
  missing  docker     not required by every repo
  created  ~/.armada  guild/, jobs/, workspaces/

Do you already have a guild?  > 3 build one now

  imported from ~/.claude/, 19 skills, 12 hooks, 4 plugins, CLAUDE.md
  projected into ~/.claude/, 20 placed

1/7  How should agents write to you?
     Tone, length, and what to lead with. Every agent reads this before it says
     anything. Writes voice.md.
     enter for a new line, ctrl-d saves, esc keeps it as it was

READY  guild at ~/.armada/guild, 0 answered, 7 kept as imported
```

**This is the record, not the menu.** Live, the guild question is a selector and each prose
question is a text area holding the current value; both are drawn on stderr and gone the moment
they close. What a transcript keeps is which option produced everything under it and what was
asked — see [`guild/init.md`](guild/init.md), which is where the interview lives, and
[`render.md`](render.md) for the widgets.

**The prompts themselves go to stderr**, on the same rule that puts progress there
([`render.md`](render.md)): stdout carries the finished transcript once, at the end, so
`armada init --json` needs no special case — the questions appear on the terminal and the
payload on stdout, and neither is in the other's way. A run with no terminal takes every
default rather than writing a prompt into a log file nobody can answer.

`--json` returns the envelope with one result per preflight check plus one for the directories
created, **and the questions that were asked** — the one verb that holds a conversation is the
one whose payload has to be an account of the same run the terminal saw.

## Dependencies

| On | Why |
|---|---|
| `git` | The guild is a git repository. |
| `claude` | Everything Fleet and Helm do runs through it. |
| Network | Only when `--guild` is used. |
| A container runtime | Optional. Warned, not enforced. |

Depends on **no other Armada command**. This is the first thing you run.

## Exit codes

`0` ready · `2` `bad_invocation` — `--guild` and `--bundle` together, or this machine is already set up and `--force` was not given · `6` `environment` — a fatal preflight check failed.

Full table and the one rule behind it: [`reference.md`](reference.md).

## See also

[`doctor.md`](doctor.md) · [`guild/init.md`](guild/init.md) · [`guild/project.md`](guild/project.md) · [`manifest/init.md`](manifest/init.md)
