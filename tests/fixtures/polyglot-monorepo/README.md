# `polyglot-monorepo` — the shape `config scan` was blind to

**This fixture exists because of a bug, and it is the bug written down.** Run
against a repository of this shape, the first `armada manifest config scan`
reported:

```
  absent  lockfile   —
  absent  scripts    —
  found   compose    docker-compose.yml, 2 services
  found   ci         .github/workflows/ci.yml, 5 steps
  absent  pyproject  —
```

Every one of those absences was wrong. The parsing was right and the **search**
was wrong: it read the root and nothing else, and the root of a monorepo holds
only the two things that describe the whole repository. Everything that
describes a *product* lives one level down.

| Directory | Holds | Why it is here |
|---|---|---|
| *(root)* | `docker-compose.yml`, `.github/workflows/ci.yml` | the only two things the blind version found — proof the parsers were never the problem |
| `web/` | `package.json`, `pnpm-lock.yaml` | a JavaScript product with its own manager and its own scripts |
| `backend/` | `pyproject.toml`, `uv.lock` | a Python product, four tool sections, versioned apart from `web/` |
| `scripts/` | `pyproject.toml`, `uv.lock` | a second Python product — **and a directory literally called `scripts`** |

That last row is a second bug in one fixture. `absent scripts —` was read as a
statement about this `scripts/` directory, when the row has only ever been about
the `scripts` block of a `package.json`. The kind is `package scripts` now, and
a kind with a space in it cannot be mistaken for a path.

**Each of the three carries its own lockfile**, which is the fact that makes
them `workspaces:` candidates ([`PLAN.md`](../../../docs/PLAN.md) §4.6): a
directory that resolves its own dependencies is a separate product sharing the
repository, not a member of somebody's workspace. Scan reports which qualify.
Deciding is still the author's job.

**It is not in `fixtures.rs`'s list and must not be**, for the same reason
`next-prisma` is not: that suite parses, validates and resolves an
`armada.yml`, and the point of both directories is that there is not one.
