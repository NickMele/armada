# `armada fleet spawn`

Start an isolated agent Job on a task.

> **Status: built — M3.** ([`PHASES.md`](../../PHASES.md) §8.5)

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
| `--confidence <0-1>` | float | `0.75` | Below this, stop and ask which workflow this is. `0` never asks. |
| `-C <path>` | directory | cwd | Which repository to branch from. |
| `--dry-run` | flag | off | Report the classification, worktree path, port block and budget. Starts nothing. |

## How it works

1. **Classify.** One cheap model call turns the task text into a workflow name, with the
   confidence surfaced so a guess is visible as a guess. Classification lives in Fleet, not the
   orchestrator, because it is needed the moment a Job can be spawned
   ([`ARCHITECTURE.md`](../../ARCHITECTURE.md) §1.9).
2. **Mint the UUID** — before anything runs. The durable handle exists before the process does,
   which is what makes ownership recordable up front and cleanup possible afterwards.
3. **Create the worktree** at `~/.armada/workspaces/<repo>/<name>` on a new branch. Plain
   `git worktree`; Fleet adds policy, not a new concept.
4. **`armada manifest init`** in it — claims a port block, runs setup. This is why parallel
   Jobs do not collide.
5. **Start a Drone, detached**, on the workflow's first step — `setsid`, its `stream-json`
   redirected to `~/.armada/jobs/<uuid>.stream.jsonl`, its process group recorded as owned.
6. **Return.** `spawn` does not wait for the turn.

**It returns while the Drone is still working, and that is the point of the verb.** A `spawn`
that ran the turn to completion could only ever run one Job at a time, and running several is
the whole of Fleet. What comes back is the handle; what the Job goes on to do is read afterwards
from its transcript by [`ls.md`](ls.md).

**A Drone is started exactly the way `armada manifest up` starts a `command` service**, and
reusing that shape is deliberate: an orphaned Drone — Armada died, the Drone did not — is reaped
by the pass that already reaps an orphaned service, so there is no second mechanism to maintain
and no second answer to *is this pid still mine*.

## Output

```
  STATUS      STEP      DETAIL                           TIME
  classified  workflow  feature, confidence 0.91         0.8s
  created     worktree  ~/.armada/workspaces/rate-limit  0.3s
  claimed     ports     5470-5479                           -
  started     drone     job 8f2a, plan step                 -

RUNNING  rate-limit, armada fleet board rate-limit to take over
```

**The confidence is on the screen and not only in the payload.** A guess has to be visible as
a guess, or nobody knows to override it. An override reports *you named it* rather than a
confidence of `1.0`: "you said so" and "the model was certain" are different facts and only
one of them is a measurement.

**Below the threshold, a spawn stops and asks.** A real spawn classified a task as `design` at
`0.10` and went straight on to make a worktree, claim a block and start a Drone on a budget.
Printing the word `a guess` is not enough when that is what happens next: §14.2 puts the
confidence on the screen *"so a guess is visible as a guess"*, and a guess visible for one line
and acted on regardless has been narrated rather than surfaced.

So the four workflows are put to you, with the model's guess already selected — `enter` confirms
it, one arrow key changes it:

```
Which workflow is this? (guessed design at 0.10)

  design   deciding an approach, no code
  plan     writing down how, before building
  feature  building something new
  bug      something is broken, reproduce it first

  up/down move · 1-4 jump · enter choose · esc keep the default
```

**What you answer is an override, not a confident model.** `confidence` is then absent from the
payload: "you said so" and "the model was certain" are different facts, and only one of them is
a measurement.

**With nobody at the terminal it refuses rather than hanging.** An agent driving Armada through
a pipe cannot answer, and a Job started on a coin flip costs a worktree, a port block and a
budget to discover — so the refusal is `bad_invocation` and names `--workflow`. The rule for
"is anybody there" is both streams being a terminal, the same one
[`manifest/config.md`](../manifest/config.md)'s hand-over uses.

**The threshold is a policy, not a tuning knob.** `0.75` is where "probably right" stops being a
reason to spend a budget unattended; a number adjusted until one task classifies one way stops
meaning anything, which is why `--confidence` moves it per spawn instead.

**The lead word on the summary line is the Job's state, not the command's.** `RUNNING` says
what the Job is doing; the envelope's `status` says how `spawn` ended, and they are different
questions ([`../../PLAN.md`](../../PLAN.md) §14.3). It is spelled in the payload under
`data.state` exactly as it is printed.

The layout is frozen by `tests/golden/render/fleet-spawn.plain` and its `.tty` twin.

`--json` returns the Job record: `uuid`, `name`, `workflow`, `confidence`, `worktree`,
`branch`, `port_block`, `budget`, `step`, `state` and `pgid`.

**There is no spend in the payload**, because there has not been one yet. The transcript is the
ledger and [`ls.md`](ls.md) reads it.

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
