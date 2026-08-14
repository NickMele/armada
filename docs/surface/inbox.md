# The inbox

How a worker that needs you actually reaches you.

> **Status: not built — M3.** Both mechanisms below were **verified in the M0 spike**
> ([`PHASES.md`](../PHASES.md) §9.1 F3).

If the orchestrator is the only thing you talk to, the system's success rests on this one
question. Get it wrong and a session sits blocked for an hour while you talk about something
else — the exact failure the design exists to prevent.

## The file

`~/.armada/inbox.jsonl`. Append-only, one JSON object per line.

```json
{"session":"nightly-flake","uuid":"…","kind":"needs_human","raised_at":"…",
 "body":"Reproduced 3/5 runs. Wants to raise CI timeout 30s → 90s.","answered":false}
```

Append-only means it survives every kind of crash, and **no daemon is involved** — the same
reasoning that put the ownership store on disk rather than in a process
([`PLAN.md`](../PLAN.md) §4.3).

## Who writes to it

| Writer | When | Reliability |
|---|---|---|
| A worker's `fleet.ask_human` MCP call | It has a question only you can answer | Requires the agent to choose to |
| A `Stop` hook | The worker's turn ended | **Fires regardless** |
| A `Notification` hook | The worker is asking for permission | **Fires regardless** |

**Hooks are the spine.** An agent can forget to report progress; it cannot forget to stop. That
distinction is what makes "needs my attention" reliable rather than best-effort, and it is why
the design does not rest on agent cooperation.

## How it reaches the orchestrator

Two mechanisms, both configuration rather than code, and deliberately overlapping.

### Live push — a plugin monitor

A monitor's every stdout line is delivered to Claude as a notification **during** the session,
so events arrive mid-turn.

```json
[{ "name": "armada-inbox",
   "command": "tail -F ~/.armada/inbox.jsonl",
   "description": "Fleet events needing you" }]
```

**Constraint:** monitors run in *interactive CLI sessions only*. That fits exactly — the
orchestrator is interactive and workers are headless — but it means a monitor can never be a
worker-side mechanism.

### Backstop — a `Stop` hook on the orchestrator

A hook returning `{"decision":"block","reason":"…"}` refuses to let the turn end while anything
is unread, and feeds the entries in. The orchestrator then finishes with *"and while we were
talking, rate-limit went green"* before handing control back to you.

The two overlap on purpose: the monitor is timely, the hook is **complete**. Neither alone
gives both.

## The gap, stated plainly

**Nothing surfaces while you are idle and silent.** If you walk away without typing, you learn
what happened when you come back. A desktop or push notification on `needs_human` closes that
gap and is independent of everything above — it needs none of this design to be finished.

## Dependencies

The guild plugin (for the monitor), the orchestrator's hook configuration, and a writable
`~/.armada/`.

## See also

[`../fleet/inbox.md`](../fleet/inbox.md) · [`orchestrator.md`](orchestrator.md) · [`../fleet/answer.md`](../fleet/answer.md)
