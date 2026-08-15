# `armada manifest skills`

What this repository knows about itself, declared once and readable by a human, a script or an
agent.

> **Status: `skills` and `skills show` are shipped. `render` is not built — M2.**
> Specified in [`PLAN.md`](../../PLAN.md) §4.8.

`commands:` ([`commands.md`](commands.md)) carries the invocation. A skill carries the judgement
around it — which of four test commands counts, what a migration name must look like here, which
column may not become nullable without a two-release plan.

## Synopsis

```sh
armada manifest skills [--json]
armada manifest skills show <name> [--json]
armada manifest render --harness <name> [--out <dir>] [--verify] [--remove] [--json]
```

## Arguments

`skills` takes nothing but `show <name>`. The rest belong to `render`, which is not built.

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `<name>` | declared skill name | — | Must appear under `skills:` in `armada.yml`. |
| `--harness <name>` | harness id | — | `render` only. Which format to render; `claude` is the only one today. |
| `--out <dir>` | directory | harness default | `render` only. Where to write. Defaults to `.claude/skills/` for `claude`. |
| `--verify` | flag | off | `render` only. Exit non-zero if the rendered output is stale. Writes nothing. |
| `--remove` | flag | off | `render` only. Delete exactly what a previous render wrote, and nothing else. |

> **`-C <path>` is reserved and not built.** A verb takes its workspace from where you are
> standing, and `cd` is the interface until something needs otherwise
> ([`config.md`](config.md)).

## How it works

A skill is two halves. The **mechanical** half is ordinary config — which commands it may run,
what verifies it, what it touches. The **prose** half is a markdown file in the repository, and
**Armada holds nothing but a path to it**. Nothing in Armada ever parses that file.

```yaml
skills:
  add-migration:
    summary: Add a Prisma migration and regenerate the client
    doc: docs/skills/add-migration.md
    uses: [migrate-new, migrate-apply]
    verify:
      check: [test, types]
    touches: ["prisma/**"]
```

**`uses:` grants nothing new.** Every name must already be declared under `commands:`, so a
skill can only ever name capability the repository declared in a file a human reviewed.

**`verify:` is what makes a skill produce a real verdict.** A Fleet verdict is only `PASS` if it
carries evidence an external command produced ([`PLAN.md`](../../PLAN.md) §14.3); naming the
check scope on the skill is what makes that automatic.

**There is no `cmd:`, and no way to run a skill.** "Add a migration" has no deterministic
expansion — `pnpm prisma migrate dev --create-only` does, and that is a `commands:` entry. A
runner would mean Armada choosing arguments on your behalf, which is exactly what the bootstrap's
layer 1 refuses to do ([`PLAN.md`](../../PLAN.md) §5).

### `render`

Writes the pair out as files a harness loads natively — for `claude`, one `SKILL.md` per skill
with generated frontmatter and the resolved commands, followed by the repository's own prose.

Output goes in a **managed region** delimited exactly as the `AGENTS.md` block is
([`PLAN.md`](../../PLAN.md) §5.1): a hand-edit outside the markers survives a re-render, and
`--remove` reverses precisely what was written, tracked by a manifest of placed files and a hash
of each.

`--verify` makes staleness an ordinary check — put `armada manifest render --harness claude
--verify` in `checks:` and drift fails the gate rather than being discovered by an agent.

> **Why a generated file rather than a committed one.** Claude Code already loads
> `.claude/skills/`, so a repository can commit skills and skip Armada entirely. What generation
> buys is that the frontmatter is accurate, the commands named are the ones the manifest actually
> declares, and the verification step is real — none of which a hand-written file can promise
> after the manifest changes underneath it.

### Verification

`armada manifest config verify` gains four cross-reference checks, all in the cheap pass:
every `doc:` exists inside the workspace root, every `uses:` names a declared command, every
`verify.check` resolves to real check ids, and no skill name shadows a built-in verb.

That is the whole argument for a schema block over a loose directory of markdown — a skill
naming a command that does not exist fails in seconds at authoring time rather than in a fresh
worktree at the worst moment.

## Output

```
armada  3d9cc7ba

  STATUS    SKILL         DETAIL
  declared  add-endpoint  Add an API endpoint, OpenAPI first, then the generate…
  declared  triage-flake  Work out whether a failing test is flaky or genuinely…

OK  2 skills, 0 unresolved references
```

`declared` is a **render-only word**, lowercase for the reason
[`render.md`](../render.md) gives: the envelope has no status that means it. Listing a skill
says the repository declares it, not that anything about it passed — whether its `uses:` and
`verify.check` resolve is `config verify`'s answer, on a different command, so a word here
that read as a verdict would claim something this one never checked.

`show` adds a second table, the same shape `status` draws its holdings with:

```
  STATUS    NAME     DETAIL
  grants    tickets  uv run scripts/tickets.py
  verifies  check    api:types
  reads     doc      docs/skills/add-endpoint.md
  touches   glob     backend/openapi.yaml
```

**The grants are only drawn for `show`**, and that is the whole reason the two views differ: at
eighty columns a listing cannot carry four columns of them and stay readable, and a table that
truncates the thing you asked for is worse than a second command.

`--json` returns one result per skill with `name`, `summary`, `doc`, `uses`, `verify` and
`touches` — with each `uses:` entry expanded to the command it names, because the one question
a reader has about a grant is what it lets the skill do. The CLI table, the MCP response and the
generated frontmatter are three renderings of one resolved structure, so a skill cannot mean one
thing to a shell caller and another to an agent.

**A grant that resolves to nothing keeps its row and is counted.** It is a `config verify`
failure, and this is not that command — but a reader looking at a list of grants should not have
to run a second one to discover that one of them names nothing.

## Dependencies

`armada.yml` with a `skills:` block, and the `commands:` and `checks:` entries it references.
`render` additionally needs a writable output directory. No network.

## Exit codes

`0` listed or written · `2` `bad_invocation` — unknown skill name · `3` `bad_config` — no
`armada.yml`, or a skill references a command or check that does not exist · `1` `tool_failed` —
`--verify` found the render stale.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`commands.md`](commands.md) · [`check.md`](check.md) · [`config.md`](config.md) ·
[`../../glossary.md`](../../glossary.md)
