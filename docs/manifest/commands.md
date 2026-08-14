# `armada manifest <repo-command>`

Run a command the repository declared. The escape hatch that stops Armada needing to know
everything.

> **Status: shipped** — the `commands:` dispatcher works today ([`PLAN.md`](../PLAN.md) §4.5).

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

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `<name>` | declared command name | — | Must appear under `commands:` in `armada.yml`. |
| `-- <args>...` | passthrough | — | Everything after `--` is appended to the command's argv, unmodified. |
| `-C <path>` | directory | cwd | Run in this workspace. |

A `commands:` entry may never shadow a built-in verb. The name is rejected at config-verify
time, not at run time, so the failure lands where it can be fixed.

## How it works

1. Resolves the workspace and its config.
2. Looks up `<name>` under `commands:`.
3. Applies templating — the four substitutions and two scoped placeholders of
   [`PLAN.md`](../PLAN.md) §4.4, which is how a declared command receives its port assignments.
4. Executes it in a new process group with the inherited environment of
   [`PLAN.md`](../PLAN.md) §2.4. Secrets are resolved into the process and never into anything
   Armada writes ([`ARCHITECTURE.md`](../ARCHITECTURE.md) §1.8).

```yaml
commands:
  seed-db:
    cmd: pnpm prisma db seed
  gen-types:
    cmd: pnpm gen:types
```

## Output

The command's own stdout and stderr, streamed. `--json` wraps the outcome — argv, exit code,
duration — but the child's output still streams, because a build log arriving only at the end
is not a build log.

## Dependencies

`armada.yml` with a `commands:` block. Whatever the command itself invokes.

## Exit codes

**The child's exit code passes through verbatim** and is never remapped, so `armada manifest test` is usable in a pipeline. Armada's own codes can only occur when the child never ran, and `data.dispatched` says which happened.

`2` `bad_invocation` — `<name>` is not declared · `3` `bad_config` — no `armada.yml`.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`config.md`](config.md) · [`check.md`](check.md)
