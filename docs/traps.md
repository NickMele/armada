# Measured behaviour

Things that are true about the environment charkit runs in, which a reasonable person would
have assumed otherwise. **Every entry here was measured, not read.**

This file exists because charkit is being built greenfield, deliberately without continuous
validation against a real repository until phase 6. That trade buys isolation and costs
empirical feedback — so the feedback that *is* obtained has to be written down or it is lost
between phases.

## How to use this file

- **Before** designing a mechanism that depends on a tool's behaviour, read the relevant
  section. If it is not covered, test it — do not infer it from documentation.
- **After** discovering something surprising, add it here with the command that demonstrates
  it, so the next reader can re-run rather than re-trust.
- An entry earns its place if believing the opposite would produce a plausible design that
  silently does not work. Entries that merely record how something works belong in the
  design documents instead.

Record for each: what was measured, the version it was measured against, the command that
shows it, and what breaks if you assume otherwise.

---

## Docker Compose

Measured against **Docker Compose v2.24.3-desktop.1**, phase 0.

### An override file appends to `ports:` — it does not replace

Base `docker-compose.yml` with `ports: ["5432:5432"]` plus an override with
`ports: ["5460:5432"]` publishes **both**.

```sh
docker compose -f docker-compose.yml -f override.yml config
# → published: "5432"   AND   published: "5460"
```

**If you assume otherwise:** you write an override to remap ports into a per-workspace block,
every workspace still binds the base port, and concurrent workspaces collide — the exact
failure charkit exists to prevent. It looks like it worked, because the new port is also
published.

### The `!override` tag needs Compose ≥ 2.24.4 and fails silently below it

```sh
# override.yml:  ports: !override ["5460:5432"]
docker compose -f docker-compose.yml -f override.yml config
# on 2.24.3 → published: "5432"    (base value, no error, no warning)
```

**If you assume otherwise:** a version floor looks like a sufficient guard. It is not, because
the failure below the floor is silent — one stale CI image or one developer on an older
Docker Desktop reintroduces the collision with nothing to indicate it.

### `docker compose config` bakes the project name into network names

Running `config` without `-p` emits `networks.default.name` derived from the *directory*, and
that value persists into any file generated from the output.

```sh
docker compose -f docker-compose.yml config | grep -A1 '^networks:'
# → name: <directory>_default
```

**If you assume otherwise:** you pass `-p char-<id>` only on the run step, and the networks
are named for whatever directory the resolve happened to run in — so ownership by project
label does not group the way you expect.

### `config` resolves `build.context` to an absolute path

```sh
docker compose -f docker-compose.yml config | grep context
# → context: /absolute/path/to/dir
```

**Useful rather than dangerous:** it is what makes it safe to emit a generated compose file
into a different directory, provided `--project-directory` is set to the original root.

### `docker compose up` has no `--label` flag

```sh
docker compose up --help | grep -c label     # → 0
```

Labels reach containers only through the compose document — `labels:` on a service, and
`build.labels:` for images the build produces. Neither `up` nor `build` accepts them on the
command line.

### Override merging does work for `labels:` and `build.labels:`

Both merge as expected. Labels were never the hard part; ports were.
