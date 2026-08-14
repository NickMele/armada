# `armada manifest config`

Author and validate `armada.yml`.

> **Status: `scan` is shipped; `verify` answers `bad_invocation` today.**

Two subcommands with one purpose between them: let an agent produce a working config for a
repository it has never seen, with no human in the loop.

## Synopsis

```sh
armada manifest config scan   [--json]
armada manifest config verify [--json]
```

## Arguments

Neither takes an argument of its own. `scan` reads the current directory and `verify` reads the
workspace the current directory resolves to.

> **`-C <path>` is reserved and not built.** Every other verb takes its directory from where you
> are standing, and one verb that also accepts a path is a second answer to "which workspace is
> this" — which is the ambiguity [`ARCHITECTURE.md`](../../ARCHITECTURE.md) §1.4 exists to
> remove. `cd` is the interface until something needs otherwise.

**`scan` is the one command that runs in a repo with no `armada.yml`** and is exempt from
workspace resolution for exactly that reason ([`PLAN.md`](../../PLAN.md) §2.1). It is answered at
the entrypoint, before resolution is attempted, because resolution would fail on precisely the
situation it exists for.

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

`scan` prints a row per kind of evidence — **present or not**, because `absent  makefile  —`
says Armada looked and a missing row says nothing at all — and then the material itself,
grouped by what produced it and **never truncated**. All fourteen scripts print: the agent
authoring the config reads this same stdout, and evidence with a `…9 more` on it is evidence
somebody has to fetch separately, which is how the one script that mattered gets missed.

It ends by offering to hand over to an agent. `ARCHITECTURE.md` §1.9 permits that — the rule
governs *inputs*, and printing a choice is an output. **Armada prints the choice and reads no
answer**; whatever acts on one is a caller above Manifest.

`--json` returns one result per finding in `data.results[]`, with the file it came from in
`path` and the one-line detail in `reason`, plus the whole uninterpreted report in
`data.evidence`. An evidence list is emitted even when it is empty, which is the opposite of
what the rest of the envelope does and for the same underlying reason: here the *kinds are the
report*, so `"makefiles": []` is how the payload says Armada looked and found none.

Two things `scan` deliberately does not report. **Confidence**, because a confidence is a
judgement and this layer has none — every value is copied out of a file the repository already
had. And **anything below the root**, apart from `.github/workflows/`: a recursive walk of a
repository nobody has configured yet is a promise about `node_modules` that nobody made.

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
