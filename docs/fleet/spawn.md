# `armada fleet spawn`

Start an isolated agent Job on a task.

> **Status: not built — M3.** ([`PHASES.md`](../PHASES.md) §8.5)

## Synopsis

```sh
armada fleet spawn "<task>" [--workflow <name>] [--name <name>] [--budget <k=v>...]
                            [-C <path>] [--dry-run] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `"<task>"` | string | — | What to do, in your words. Required. |
| `--workflow <name>` | workflow id | **classified** | Override classification. Names a file in `~/.armada/guild/workflows/`. |
| `--name <name>` | string | derived from task | The Job's handle. Must be unique among live Jobs. |
| `--budget <k=v>` | repeatable | from the workflow | Override one ceiling, e.g. `--budget max_tokens=200000`. |
| `-C <path>` | directory | cwd | Which repository to branch from. |
| `--dry-run` | flag | off | Report the classification, worktree path, port block and budget. Starts nothing. |

## How it works

1. **Classify.** One cheap model call turns the task text into a workflow name, with the
   confidence surfaced so a guess is visible as a guess. Classification lives in Fleet, not the
   orchestrator, because it is needed the moment a Job can be spawned
   ([`ARCHITECTURE.md`](../ARCHITECTURE.md) §1.9).
2. **Mint the UUID** — before anything runs. The durable handle exists before the process does,
   which is what makes ownership recordable up front and cleanup possible afterwards.
3. **Create the worktree** at `~/.armada/workspaces/<repo>/<name>` on a new branch. Plain
   `git worktree`; Fleet adds policy, not a new concept.
4. **`armada manifest init`** in it — claims a port block, runs setup. This is why parallel
   Jobs do not collide.
5. **Start a bounded headless turn** with the workflow's first step and the guild's content.
6. **Register the Job** in `~/.armada/jobs/` — uuid, name, worktree, branch, port block,
   budget.

## Output

```
feature (0.94)                    override with --workflow
rate-limit  → ~/.armada/workspaces/api/rate-limit  ports 41210–41219
            → budget 12 iterations · 400k tokens · 45m
```

`--json` returns the Job record: `uuid`, `name`, `workflow`, `confidence`, `worktree`,
`branch`, `port_block`, `budget`.

## Dependencies

| On | Why |
|---|---|
| A git repository | Worktrees need one. |
| `armada.yml` | For the `manifest init` step. |
| An initialised guild | Workflows and agent content come from it. |
| `claude` | Runs the Job. |

## Exit codes

`0` spawned · `1` `tool_failed` — the worktree or `manifest init` failed · `2` `bad_invocation` — unknown workflow, or the Job name is taken · `3` `bad_config` — no `armada.yml` · `6` `environment` — not a git repository, or the port pool is exhausted.

**A failed spawn cleans up after itself.** A half-created worktree holding a claimed port block is released before the error returns.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`ls.md`](ls.md) · [`kill.md`](kill.md) · [`../helm/helm.md`](../helm/helm.md)
