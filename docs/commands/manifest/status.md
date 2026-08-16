# `armada manifest status`

What is running, what is mine, what is stale.

> **Status: shipped.**

## Synopsis

```sh
armada manifest status [--project|--all] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `--project` | flag | off | Every workspace of this repository, not just this one. |
| `--all` | flag | off | Every workspace on this machine. |

`--project` and `--all` are two different scopes; pass one. **There is no `--stale`** — stale
resources are reported in every mode, and the flags are lenses on how wide to look, not on what
to report.

> **`-C <path>` is reserved and not built.** A verb takes its workspace from where you are
> standing, and `cd` is the interface until something needs otherwise
> ([`config.md`](config.md)).

## How it works

Reads the machine-global store and reconciles it against reality: for each recorded resource it
checks whether the thing still exists. That reconciliation is the point — a store that is
trusted blindly drifts, and a filesystem scan alone cannot attribute what it finds.

Three states per resource: **live** (recorded and present), **stale** (recorded and gone), and
**unowned** (present, and not recorded by any workspace).

**Every workspace the store holds any state for, not only the ones that claimed ports.** A
`workspaces` row records a *port claim* and is written by `init` alone; an `owned` row records a
*resource that exists* and is written by `up`, by `check --detach` and by `fleet spawn`, none of
which need a claim. A repository that declares no `ports:` therefore accumulates owned resources
against a workspace the registry has never heard of — measured, on Armada's own checkout, at six
leaked process groups this verb could not see
([`reserved/023`](../../reserved/023-status-shows-what-is-running.md)). The workspace you are
standing in always gets a row, even when the store holds nothing for it: *nothing owned* and
*nobody looked* must not read the same.

**Detached runs are reported while they are running.** `armada manifest check --detach` returns
immediately, and until this landed the only way to see the run was `check --status` with its id
in hand. `status` names the runs that have **not reached a verdict**, from this boot, as
`RUNNING` or `DEAD` — the same two words `check --status` uses for the same two facts. A run that
decided is history and is left out; `check --status <id>` reads a verdict back.

**Staleness is proved, never suspected.** A recorded process group is `stale` only under the rule
`clean` kills on — a boot id that is not this boot, or a start time that no longer matches — so
a stale row is an instruction rather than a worry. Containers, networks, volumes and images are
never judged here: deciding would take a daemon call, and this verb asks no daemon.

## Output

```
armada  3d9cc7ba  ~/work/api                                    ports 5460-5469

  STATUS   RUN               DETAIL
  RUNNING  01M048YQMSD6YP48  pgid 4212 · .armada/run/01M048YQMSD6YP48/detach.log

  STATUS    RESOURCE   DETAIL
  STALE     pgid       4098
  OWNS      container  armada-3d9cc7ba-api
  OWNS      volume     pgdata
  HELD      lease      run:3d9cc7ba
  REPORTED  release    psql -c 'DROP DATABASE app_3d9cc7ba'

OK  1 workspace, scope workspace
```

Stale rows are drawn first, because only three resources are named before the rest collapse into
a count — and an ordering that let the one row you can act on fall into `+2 more` would hide it
behind three that need nothing.

`--json` returns one result per workspace carrying `owns[]`, `stale[]` (a subset of `owns[]`, in
the same `<kind>:<reference>` grammar), `runs[]`, `ports`, `leases[]` and the port block.
Read-only in every mode.

## Dependencies

`~/.armada/manifest.db`, plus a container runtime to reconcile container-backed resources.
Works without `armada.yml`.

## Exit codes

`0` whenever the store could be read — status reports, it does not judge, so a stale resource is *not* a non-zero exit · `6` `environment` — the store is unreadable.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`clean.md`](clean.md) · [`explain.md`](explain.md)
