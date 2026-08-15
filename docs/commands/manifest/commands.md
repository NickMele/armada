# `armada manifest commands`, and `armada manifest <name>`

List the commands a repository declared, and run one. The escape hatch that stops Armada
needing to know everything, plus the listing that stops you needing to open the file.

> **Status: shipped.**
> Both halves work — the dispatcher ([`PLAN.md`](../../PLAN.md) §4.5) and the listing that
> section reserved.

Every repo has scripts Armada has no opinion about: seeding a database, generating types,
deploying to staging. Declaring them in `armada.yml` means **an agent working in an unfamiliar
repo does not have to re-derive them from a README**, and the invocation is identical
everywhere.

## Synopsis

```sh
armada manifest <name> [-- <args>...] [-C <path>] [--json]
armada manifest commands            # list what this repo declares
```

## Arguments

`commands` takes nothing. A listing is not itself a thing to filter, and a caller who wants
one entry runs it.

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `<name>` | declared command name | — | Must appear under `commands:` in `armada.yml`. `armada manifest commands` is how you learn which names those are. |
| `-- <args>...` | passthrough | — | Everything after `--` is appended to the command's argv, unmodified. |
| `-C <path>` | directory | cwd | Run in this workspace. |

A `commands:` entry may never shadow a built-in verb. The name is rejected at config-verify
time, not at run time, so the failure lands where it can be fixed.

**`commands` is one of those names now, and taking it cost every repository one.** The verb
that lists a `commands:` block is the single name that block may no longer contain. That is the
trade a promoted name always carries ([`PLAN.md`](../../PLAN.md) §4.5), and it is worth it here
because "what can I run in this repo" had no other answer: the names lived in `armada.yml` and
nothing printed them, so `armada --help`'s `<name>` row named a placeholder a reader could only
resolve by opening the file.

## How it works

1. Resolves the workspace and its config.
2. Looks up `<name>` under `commands:`.
3. Applies templating — the four substitutions and two scoped placeholders of
   [`PLAN.md`](../../PLAN.md) §4.4, which is how a declared command receives its port assignments.
4. Executes it in a new process group with the inherited environment of
   [`PLAN.md`](../../PLAN.md) §2.4. Secrets are resolved into the process and never into anything
   Armada writes ([`ARCHITECTURE.md`](../../ARCHITECTURE.md) §1.8).

```yaml
commands:
  seed-db:
    cmd: pnpm prisma db seed
  gen-types:
    cmd: pnpm gen:types
```

## Output

**`commands`** draws the same three columns `skills` and `components` do, because it answers the
same kind of question about the same document:

```
armada  5bc3158e

  STATUS    COMMAND    DETAIL
  DECLARED  deploy     Deploy this branch to staging
  DECLARED  seed-db    pnpm prisma db seed
  DECLARED  worktrees  Create and tear down git worktrees

OK  3 commands, 1 with a secrets grant

`armada manifest <command>` runs one; everything after the name is its own.
These are this repository's own verbs; `armada manifest --help` lists Armada's.
```

`DECLARED` says the repository declares the entry — not that `argv[0]` exists, not that its
grant resolves. Those are `config verify`'s answers, on a different verb.

**An entry with no `help:` shows its `cmd:` instead**, muted rather than blank: a blank cell
reads as a defect, and the command string is the only other thing Armada knows about the entry.

**How many entries can reach a secret is a count in the summary, not a status word.** It is a
fact about the listing rather than a state of any row, and it is the same question
`grep -n "secrets:"` used to answer.

**A repository that declares none gets different lines rather than the same lines over an empty
table** — it is told what an entry is made of, since telling it how to run one of nothing says
nothing.

`--json` returns one result per entry plus a `commands[]` carrying `name`, `cmd`, `help`,
`stdio` and `secrets`. **`stdio` is the resolved value, after inference** — an entry with a
grant and no `stdio:` key reports `pipe`, and nothing in `armada.yml` says so. The envelope's
`verb` is `commands` here and on a dispatch; the two are told apart by their bodies.

**Dispatching** streams the command's own stdout and stderr. `--json` wraps the outcome — argv,
exit code, duration — but the child's output still streams, because a build log arriving only at
the end is not a build log.

## Dependencies

`armada.yml` with a `commands:` block. Whatever the command itself invokes.

## Exit codes

**The child's exit code passes through verbatim** and is never remapped, so `armada manifest test` is usable in a pipeline. Armada's own codes can only occur when the child never ran, and `data.dispatched` says which happened.

`commands`: `0` listed · `3` `bad_config` — this workspace has no readable `armada.yml`. A read
verb: its exit code describes the query, not what it found.

Dispatch: `2` `bad_invocation` — `<name>` is not declared · `3` `bad_config` — no `armada.yml`.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`config.md`](config.md) · [`components.md`](components.md) · [`../../armada-yml.md`](../../armada-yml.md) — every key the file accepts
