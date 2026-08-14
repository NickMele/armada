# `armada manifest config`

Author and validate `armada.yml`.

> **Status: not built.** Answers `bad_invocation` today.

Two subcommands with one purpose between them: let an agent produce a working config for a
repository it has never seen, with no human in the loop.

## Synopsis

```sh
armada manifest config scan   [-C <path>] [--json]
armada manifest config verify [-C <path>] [--json]
```

## Arguments

| Subcommand | Flag | Meaning |
|---|---|---|
| `scan` | `-C <path>` | Directory to scan. |
| `verify` | `-C <path>` | Workspace whose `armada.yml` to validate. |

**`scan` is the one command that runs in a repo with no `armada.yml`** and is exempt from
workspace resolution for exactly that reason ([`PLAN.md`](../../PLAN.md) §2.1).

## How it works

**`scan`** reads evidence and reports it. Roughly a dozen independent parsers — package
manifests, lockfiles, compose files, CI workflows, test and lint configuration — each
contributing what it found. It **emits evidence, not a finished config**: the author (you, or
an agent) turns evidence into a config, because a scanner that guesses produces a file nobody
can trust and everybody has to re-read.

**`verify`** validates a written `armada.yml` against the schema and the cross-key rules that
a schema cannot express: `needs:` referring to things that exist, no `commands:` entry
shadowing a built-in verb, `owns:` keys matching their driver.

The intended loop is `scan` → author → `verify`, iterating until verify is clean.

## Output

`scan` prints the evidence grouped by what produced it. `--json` returns one result per finding
with its source file and confidence.

`verify` prints one line per problem, each naming the key path and what would fix it. Clean
verification prints one line.

## Dependencies

`scan` depends on nothing but a readable directory. `verify` needs `armada.yml`.

## Exit codes

`scan`: `0` whenever the directory is readable — it reports rather than judges.

`verify`: `0` valid · `3` `bad_config` — invalid, or no `armada.yml`. `next_action` is populated on every finding, because `bad_config` requires it.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`init.md`](init.md) · [`commands.md`](commands.md)
