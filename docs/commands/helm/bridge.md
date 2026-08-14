# `armada bridge`

The live screen. Every Job, its state, what it has spent, and who needs you.

> **Status: not built — M3.** Scheduled alongside Helm and Fleet. An earlier draft deferred the
> ambient view indefinitely; [`PLAN.md`](../../PLAN.md) §15.1 records why that was reversed.

Helm is where you talk. The Bridge is what you watch. It redraws in place like `htop` or `k9s`
— no scrollback, no history, just the current state of the fleet.

## Synopsis

```sh
armada bridge [--filter <expr>] [--interval <s>] [--once] [--json]
```

`/bridge` from inside a Helm session opens the same screen.

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `--filter` | expression | — | Show only matching Jobs. Same selector grammar as [`../fleet/ls.md`](../fleet/ls.md). |
| `--interval` | seconds | `2` | Redraw cadence. Reads are cheap — no process is interrupted. |
| `--once` | flag | off | Render one frame and exit. For a screenshot, a pipe, or a terminal that cannot hold alt-screen. |
| `--json` | flag | off | Emit one frame as the `--json` envelope and exit. Implies `--once`. |

## How it works

Reads the Job index in `~/.armada/jobs/` and, per Job, the tail of its transcript at
`~/.claude/projects/<slug>/<uuid>.jsonl`. **Every column comes from data Claude Code already
emits** — the turn's `result` event carries `total_cost_usd`, `usage`, `num_turns` and
`duration_api_ms` ([`PHASES.md`](../../PHASES.md) §9.1 F2). The Bridge builds no accounting layer
and estimates nothing.

**Read-only, always.** It never resumes, interrupts, or probes a Drone. Watching something must
not change it — the same rule that governs probe ([`PLAN.md`](../../PLAN.md) §15.2).

**It holds no state.** The Bridge is a renderer over Fleet; closing it loses nothing, and
everything it shows is available from [`../fleet/ls.md`](../fleet/ls.md) as a table.

### Columns

| Column | Source | Notes |
|---|---|---|
| `JOB` | the Job index | The name, not the uuid — names are assigned at spawn and are what you type. |
| `STATE` | Fleet Job state | `QUEUED` `RUNNING` `PAUSED` `STALLED` `BLOCKED` `ABORTED` `DONE` ([`../glossary.md`](../../glossary.md)). |
| `TASK` | the spawn prompt | Truncated. The Bridge is a status view, not a reader. |
| `RUN` | wall clock since spawn | — |
| `SPENT` | `total_cost_usd`, summed over turns | Against the Job's ceiling, so exhaustion is visible before it happens ([`PLAN.md`](../../PLAN.md) §14.3). |
| `NEEDS YOU` | the inbox | The only column that is ever a call to action. |

> **There is no progress column, deliberately.** Nothing emits percent-complete: F2 gives cost,
> tokens, turns and duration, none of which is progress toward a goal an agent has not finished
> defining. A bar computed from turn count would be a confident guess rendered as a measurement,
> which is worse than no column at all.

### Keys

| Key | Does | Verb it calls |
|---|---|---|
| `↵` | Board the selected Job | [`../fleet/board.md`](../fleet/board.md) |
| `n` | New Job | [`../fleet/spawn.md`](../fleet/spawn.md) |
| `p` | Pause / resume | — |
| `x` | Abort | [`../fleet/kill.md`](../fleet/kill.md) |
| `a` | Answer the selected Job's question | [`../fleet/answer.md`](../fleet/answer.md) |
| `/` | Filter | — |
| `c` | Drop into Helm | [`helm.md`](helm.md) |

Every key maps to a verb that already exists and is reachable from a shell. **The Bridge adds no
capability**, which is what keeps it a rendering choice rather than an architectural one.

## Output

```
┌─ ARMADA BRIDGE ─────────────────────────────────────── ● LIVE ─┐
│ running 2   blocked 1   spent today $8.40                      │
├──────────────┬─────────┬────────────────────┬─────┬───────┬────┤
│ JOB          │ STATE   │ TASK               │ RUN │ SPENT │ ●  │
├──────────────┼─────────┼────────────────────┼─────┼───────┼────┤
│ rate-limit   │ RUNNING │ add gateway limiter│ 14m │ $2.10 │    │
│ carina-schema│ RUNNING │ migrate schema     │  3m │ $0.45 │    │
│ xlsx-report  │ STALLED │ generate report    │ 22m │ $4.60 │    │
│ release-merge│ BLOCKED │ merge release      │ 1h  │ $1.25 │ ●  │
└──────────────┴─────────┴────────────────────┴─────┴───────┴────┘
 ↵board  n new  p pause  x abort  a answer  /filter  c chat
```

`--json` returns one result per Job with the same fields as
[`../fleet/ls.md`](../fleet/ls.md), so a frame and a listing parse identically.

## Palette

One palette, shared with every coloured surface Armada renders, defined in
[`../render.md`](../render.md).

## Dependencies

Fleet's Job index and the Claude Code transcripts. No network, no daemon, no repository — it
runs anywhere `~/.armada/` is readable.

## Exit codes

`0` clean exit · `2` `bad_invocation` — an unparseable `--filter` · `6` `environment` — the
terminal cannot support alt-screen and `--once` was not given.

Full table and the one rule behind it: [`../reference.md`](../reference.md).

## See also

[`helm.md`](helm.md) · [`../fleet/ls.md`](../fleet/ls.md) · [`inbox.md`](inbox.md) ·
[`../glossary.md`](../../glossary.md)
