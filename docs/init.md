# `armada init`

Set up **this machine**. Run once per box.

> **Status: not built — M2.** ([`PHASES.md`](PHASES.md) §8.4)

Not to be confused with [`manifest/init.md`](manifest/init.md), which sets up a *workspace*.
This sets up *you, here*.

## Synopsis

```sh
armada init [--guild <remote>] [--bundle <path>] [--no-interview] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `--guild <remote>` | git remote URL | — | Skip the prompt and pull an existing guild from this remote. |
| `--bundle <path>` | file path | — | Skip the prompt and import an existing guild from a bundle. |
| `--no-interview` | flag | off | Run the checks and create an empty guild. For scripted setup; you configure it later with [`guild/edit.md`](guild/edit.md). |
| `--force` | flag | off | Re-run against an existing `~/.armada/`. Refuses without this. |

`--guild` and `--bundle` are mutually exclusive.

## How it works

1. **Preflight.** Checks for `git`, `claude`, and a container runtime. Reports each as present
   or missing with the version found. Missing `claude` is fatal; a missing container runtime is
   a warning, because not every repo needs one.
2. **Creates `~/.armada/`** — `guild/`, `jobs/`, `workspaces/`, `manifest.db`,
   `machine.yml`.
3. **Asks the one question that matters:** *do you already have a guild?*
   - **Pull from a remote** — clones it into `~/.armada/guild/`. Done in seconds; this is the
     second-machine path.
   - **Import a bundle** — unpacks a file. For a machine that will never hold your credentials.
   - **Build one now** — hands off to [`guild/init.md`](guild/init.md).
4. **Writes `machine.yml`** — paths, capacity, and anything machine-specific. This file
   **never syncs** ([`PLAN.md`](PLAN.md) §13.1).

## Output

Human-readable: a checklist, then the guild summary.

```
✓ git 2.51    ✓ claude 2.x    ✓ docker running
Guild pulled from <remote>: 19 skills · 12 hooks · 4 workflows · voice, expectations, how-i-work
Ready.
```

`--json` returns the envelope with one result per preflight check plus one for the guild, each
carrying `status` and the version or path found.

## Dependencies

| On | Why |
|---|---|
| `git` | The guild is a git repository. |
| `claude` | Everything Fleet and Helm do runs through it. |
| Network | Only when `--guild` is used. |
| A container runtime | Optional. Warned, not enforced. |

Depends on **no other Armada command**. This is the first thing you run.

## Exit codes

`0` ready · `2` `bad_invocation` — `--guild` and `--bundle` together, or `~/.armada/` exists without `--force` · `6` `environment` — a fatal preflight check failed.

Full table and the one rule behind it: [`reference.md`](reference.md).

## See also

[`doctor.md`](doctor.md) · [`guild/init.md`](guild/init.md) · [`manifest/init.md`](manifest/init.md)
