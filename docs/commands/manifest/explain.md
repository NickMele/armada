# `armada manifest explain`

Hand back the evidence a stack trace does not carry.

> **Status: not built.** Answers `bad_invocation` today.

Armada runs no model. Its caller is already an agent, and the useful thing Armada can do is
give that agent **what it cannot see**: the exact argv, what a check waited on and who held it,
and whether this failure has happened before ([`PLAN.md`](../../PLAN.md) §3.4).

## Synopsis

```sh
armada manifest explain [<check-id>] [-C <path>] [--history <n>] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `<check-id>` | check id | last failure | What to explain. |
| `--history <n>` | integer | 3 | How many previous runs of the same check to compare against. |
| `-C <path>` | directory | cwd | Which workspace. |

## How it works

Reads the dispatch record written by [`check.md`](check.md) and [`up.md`](up.md) at the moment
they ran, and assembles four things:

1. **The exact argv**, including templated substitutions, as executed.
2. **The wait graph** — what it blocked on, who held the lock or lease, and for how long.
3. **The bind state at dispatch** — which ports were assigned and which were in use.
4. **The failure signature**, compared against the last `n` runs.

The history row is the part nothing else provides: **two runs of the same bug produce the same
signature, and a different bug in the same check produces a different one.** That is what tells
an agent whether it is looking at a flake or a regression, and it changes what the agent does
next.

## Output

Prose for a human, the same content structured for `--json`:

```
check   test
argv    pnpm vitest run --reporter=json
waited  4.2s on lease `db` held by workspace api (a3f2c1)
ports   41203 assigned, bound at dispatch
history 3/3 previous runs failed with the same signature — not a flake
```

## Dependencies

The dispatch record, which only exists if `check` or `up` has run in this workspace. Explaining
a check that has never run reports exactly that, rather than an empty result.

## Exit codes

`0` evidence returned · `2` `bad_invocation` — unknown check id · `3` `bad_config` — no workspace.

**No dispatch record is not an error.** Explaining a check that has never run exits `0` and says so, because "it never ran" is itself the evidence the caller needed.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`check.md`](check.md) · [`status.md`](status.md)
