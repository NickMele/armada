# `armada manifest components`

What this repository can be filtered by.

> **Status: shipped.**

## Synopsis

```sh
armada manifest components [--json]
```

## Arguments

Takes nothing. Listing what a repository can be filtered by is not itself a thing to filter,
and a caller who wants one component's detail is asking a verb that acts on it.

## Why it exists

`armada manifest check --component <name>` has always taken a name, and there was no way to
learn what the names were except by opening `armada.yml`. That is the first question anyone
asks of a repository they have not seen — human or agent — and parsing YAML to answer it is
exactly the work Armada exists to remove.

`skills:` had the same gap and [`skills.md`](skills.md) closed it. This is the same answer in
the same shape, and [`PLAN.md`](../../PLAN.md) §4.5's reserved `armada manifest commands` is
the third.

**Three columns, because there are three things a caller does next.** The name is what
`--component` takes. `RUNS` says the component declares a `run:`, so `up` and `down` act on
it. The checks are what `<component>:<check>` selects.

Everything else about a component — its `setup:`, its `needs:`, what it owns — belongs to a
verb that acts on it rather than to the list you read before choosing.

## Output

```
armada  3d9cc7ba

  STATUS    COMPONENT  CHECKS
  RUNS      api        lint, test
  DECLARED  docs       —
  DECLARED  web        e2e, lint, types

OK  3 components, 5 checks

`armada manifest check --component <name>`, or `<name>:<check>` for one check.
```

`DECLARED` is not a lesser state than `RUNS` and is not painted as one: a component that is
only a set of checks is ordinary and common. The word says which of `up` and `check` this row
is reachable by, and nothing about whether anything passed.

`--json` returns one result per component with `name`, `root`, `runs` and `checks` — structure
rather than a rendered table, so an agent lists a repository's shape without reading YAML.

## The consequence that landed with it

**No repository may declare a `commands:` entry or a skill named `components`.**
[`PLAN.md`](../../PLAN.md) §4.5's shadow rule applies the moment a name is taken, and the
schema rejects it. That is the trade a promoted name always carries: a name in Armada's
namespace is a name taken from every repository. It is worth it here because "what can I filter
by" has no other answer.

Closing this also closed two the schema had been letting through. `skills` and `render` were
built-in verbs and absent from the forbidden list, so a repo could declare them and have
Armada's verb silently shadow theirs — the exact failure §4.5 exists to prevent. The list is
now checked against the parser's own by a test rather than kept in step by hand.

## Exit codes

`0` listed · `3` `bad_config` — this workspace has no readable `armada.yml`.

A read verb: its exit code describes the query, not what it found.
