# `next-prisma` — the fixture `armada manifest config scan` is drawn from

**The one fixture with no `armada.yml`, and that is the axis it owns.** The six
configs beside it (`PHASES.md` §8.1) all describe a workspace that has already
been authored; this one is the repository *before* layer 2 has happened, which
is the only state `config scan` ever runs in ([`PLAN.md`](../../../docs/PLAN.md)
§5).

It is the shape the agreed layout was drawn against
(`docs/reference-output/command-output.html`): a pnpm application with a
lockfile, fourteen scripts, three compose services and a CI workflow, and
deliberately **no** `pyproject.toml` and no `Makefile` — so the evidence table
has to draw the two `absent` rows as well as the four `found` ones.

| File | What it makes the scan report |
|---|---|
| `package.json` | fourteen scripts, in the order the file writes them, and the `pnpm@9` pin |
| `pnpm-lock.yaml` | the lockfile row. Its contents are never read |
| `docker-compose.yml` | three services, one of them publishing two ports |
| `.github/workflows/ci.yml` | six steps, four of which run a command |

**It is not in `fixtures.rs`'s list and must not be.** That suite parses,
validates and resolves an `armada.yml`, and the whole point of this directory is
that there is not one.
