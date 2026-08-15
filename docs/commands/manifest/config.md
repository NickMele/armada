# `armada manifest config`

Author and validate `armada.yml`.

> **Status: shipped.**

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

### Where it looks

**Not just the root.** The first version read the root and nothing else, and on a monorepo that
made it blind to the thing it most needed to see: run against a real polyglot repository it
reported `absent lockfile` and `absent scripts` while `web/pnpm-lock.yaml`, `backend/uv.lock`
and `backend/pyproject.toml` sat one level down. The parsing was right and the *search* was
wrong.

It descends, and the bound is the whole design:

| Bound | Why |
|---|---|
| **`.gitignore`, via git** | A build output is not a package, and git already knows which is which. `git ls-files --cached --others --exclude-standard` is the listing whenever there is a checkout to ask, so the answer is the repository's own rules rather than a hand-rolled subset of them. |
| **A refused-directory list** | The same answer for a directory that is not a checkout — `scan` runs before anything is set up — and for the repository that commits its `vendor/` tree, where git would say it is not ignored and the evidence would still be somebody else's. |
| **Three levels** | `web/` is one, `apps/web/` and `crates/core/` are two, and three is unusual but real. At four, what a scan finds stops being packages: a vendored copy, a fixture, an example app inside a library. |

**Nothing is followed through a symlink**, which is how a bounded walk becomes an unbounded one.

### Location is evidence

`backend/` holding `uv.lock` and `pyproject.toml` while `web/` holds `pnpm-lock.yaml` is the
single most important fact about such a repository — it is three products in one checkout — and
a flat "found: lockfile" list destroys it. So every path in the report is workspace-relative and
complete, and the **`packages`** section states the grouping outright.

**`workspaces:` candidates are reported and never decided.** The fact underneath is on disk: a
directory with a manifest *and* a lockfile of its own resolves its own dependencies, which is
what tells a separate product from a member of somebody's workspace — a pnpm workspace member
has the manifest and no lockfile, because the root resolved for it. Separate products sharing
one repository is exactly what `workspaces:` is for
([`PLAN.md`](../../PLAN.md) §4.6). Whether to declare one is the author's call; layer 1 says
which qualify.

> **The kind is `package scripts`, not `scripts`.** A repository with a `scripts/` directory
> that is a Python package had its `absent scripts —` row read as a statement about that
> directory. The row has only ever been about the `scripts` block of a `package.json`, and a
> kind with a space in it cannot be mistaken for a path.

**`verify` runs in two passes, and only the first is cheap.**

| Pass | What it does |
|---|---|
| **1 — static** | schema, references, `argv[0]` resolvability, glob coverage. Seconds. Nothing is executed. **Failures short-circuit here.** |
| **2 — for real** | runs the check suite exactly as `armada manifest check` does. Only attempted if pass 1 passed. |

Pass 1 draws four rows. Three are checks and carry a verdict:

| Row | What it establishes |
|---|---|
| `schema` | The document validates against the `armada.yml` JSON Schema **and** resolves through the structs. Both, because they are the same contract at two entry points and not the same strength: every value-level rule — the name grammar, a port's range, `shell: true` beside `${files}` — belongs to the schema alone. |
| `references` | `needs:` resolves and is acyclic; `match:` hits a tracked file; `in:` names a compose component; ports fit the block and no name is declared twice; no `root:` or glob escapes the workspace or reaches into a declared nested one; every granted secret is declared and every scheme has a provider; and every skill's `doc:`, `uses:` and `verify.check` resolve. |
| `argv[0]` | Every argv-split `cmd:`, `fix:`, `setup:` step and `run.cmd` names something on `PATH`, or an executable file under the component root. |

The fourth is **`unchecked`**, and it is not a verdict. Entries under `shell: true` have no
`argv[0]` to resolve — the string is a program in a language Armada does not parse, and `VAR=x
exec "$TOOL"` has no first word that is a command — so they are **counted, never guessed at and
never silently passed**. That count is the honest cost of the key and is worth seeing, which is
why it has a row rather than a footnote.

**Pass 2 is a real run, not a simulation.** An earlier draft had verify dry-invoke every `cmd:`
with `--help` / `--version` / `--dry-run`, which was the worst of both worlds: Armada cannot know
which of the three a given tool accepts, so against the fixture set it would either run the
Playwright suite, create a Kubernetes cluster, or fail a correct config. Guessing a flag is not
verification. If you want to know a config works, run it.

**Consequence, stated plainly:** `verify` is *not* a seconds-long command. Pass 1 is, and it
catches the hallucinated script name that motivated this layer; pass 2 takes as long as the
repository's checks take. An authoring loop iterating on pass-1 failures stays fast, and a full
verify is a build. It inherits `check`'s semantics wholesale, including that **verify does not
stop what it started**.

The intended loop is `scan` → author → `verify`, iterating until verify is clean.

## Output

`scan` prints a row per kind of evidence — **present or not**, because `absent  makefile  —`
says Armada looked and a missing row says nothing at all — and then the material itself,
grouped by what produced it and **never truncated**. All fourteen scripts print: the agent
authoring the config reads this same stdout, and evidence with a `…9 more` on it is evidence
somebody has to fetch separately, which is how the one script that mattered gets missed.

**Every section is keyed by the file or the directory it came from**, and a repository with two
compose files gets two sections rather than one merged list — the first version merged them and
printed `postgres` and `redis` twice with nothing saying which file either came from, which is
the same fact-destroying merge one level down.

The `packages` and `workspaces: candidates` sections are drawn **only when there is a layout to
draw**. A repository with one package at the root has no monorepo structure, and a section
saying `.  package.json, pnpm-lock.yaml` restates the table above it. `--json` carries
`packages` either way, because a consumer asking "what is here" wants the answer whichever
shape the repository is.

The one line that could grow without bound — the CI steps — **wraps rather than truncating**,
and it is the only place the renderer wraps anything. Both of the usual answers are wrong here:
a flexible column would drop the tail, and one line would run to seven hundred columns on a
repository whose gate has a dozen steps.

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

`verify` prints its pass-1 table, then pass 2's if it ran, then a verdict — and under the
verdict **one `->` line per problem, each naming what would fix it.** Those lines are the point:
a report that names a problem without the command that fixes it sends the reader to the
documentation, which is most of what this verb exists to save. The key path to edit is in
`results[].error.where` and in the `--json` payload.

A pass-1 failure short-circuits, and the render says so by omission: the rows for checks that
did not run are **absent** rather than present with some third status, because a row claiming
they ran would be a claim about work nobody did. `data.pass_2` is likewise absent rather than
empty — *not attempted* is not *skipped*.

## Dependencies

`scan` depends on nothing but a readable directory. `verify` needs `armada.yml`, and pass 2
needs whatever the repository's checks need.

## Exit codes

`scan`: `0` whenever the directory is readable — it reports rather than judges.

`verify`: `0` valid · `3` `bad_config` — invalid, or no `armada.yml`. `next_action` is populated on every finding, because `bad_config` requires it. When pass 2 runs, its own verdict is the answer, so a config that verifies but whose tests fail exits `1` `tool_failed` — the config was right and the tests were not, which is a different action.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`init.md`](init.md) · [`commands.md`](commands.md)
