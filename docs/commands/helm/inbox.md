# The inbox

How a Drone that needs you actually reaches you.

> **Status: built — M3.** Both mechanisms were **verified in the M0 spike**
> ([`PHASES.md`](../../PHASES.md) §9.1 F3), and [`armada helm`](helm.md) now writes both. They
> are still configuration rather than code: what Armada produces is a plugin directory and a
> settings file, and neither is a process Armada runs.

If the orchestrator is the only thing you talk to, the system's success rests on this one
question. Get it wrong and a Job sits blocked for an hour while you talk about something
else — the exact failure the design exists to prevent.

## The file

`~/.armada/inbox.jsonl`. Append-only, one JSON object per line.

```json
{"type":"raised","uuid":"…","job_uuid":"…","job":"nightly-flake","kind":"needs_human",
 "raised_at":"…","raised_ms":…,"body":"Reproduced 3/5 runs. Wants CI timeout 30s → 90s."}
{"type":"answered","uuid":"…","answer":"yes, 90s"}
{"type":"closed","uuid":"…","why":"ended"}
```

Append-only means it survives every kind of crash, and **no daemon is involved** — the same
reasoning that put the ownership store on disk rather than in a process
([`PLAN.md`](../../PLAN.md) §4.3).

**An answer is its own line, so "unread" is not a property of any single line.** A reader folds
the lines that share a `uuid`: a `raised` line with no later `answered` or `closed` line, and a
`job_uuid` to resolve against, is what open means. This page used to show a single object with
an `"answered":false` field, and there has never been one — that fiction is what the `Stop` hook
below was greping for.

## Who writes to it

| Writer | When | Reliability |
|---|---|---|
| A Drone's `fleet.ask_human` MCP call | It has a question only you can answer | Requires the agent to choose to |
| A `Stop` hook | The Drone's turn ended | **Fires regardless** |
| A `Notification` hook | The Drone is asking for permission | **Fires regardless** |

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
   "command": "tail -F /Users/you/.armada/inbox.jsonl",
   "description": "Fleet events needing you" }]
```

Written to `~/.armada/helm/plugin/monitors/monitors.json` and loaded with `--plugin-dir`, which
needs no marketplace and no install step. **The path is absolute rather than `~/…`**: a
monitor's command is not necessarily run through a shell that expands a tilde, and a monitor
tailing a file called `~` reports nothing forever.

**Constraint:** monitors run in *interactive CLI sessions only*. That fits exactly — the
Helm is interactive and Drones are headless — but it means a monitor can never be a
Drone-side mechanism.

### Backstop — a `Stop` hook on the orchestrator

A hook returning `{"decision":"block","reason":"…"}` refuses to let the turn end while anything
is unread, and feeds the entries in. The orchestrator then finishes with *"and while we were
talking, rate-limit went green"* before handing control back to you.

Written to `~/.armada/helm/stop-inbox.sh` and registered by `--settings` — **for that session and
not for the machine**. The same hook in `~/.claude/settings.json` would fire in every session on
the machine, including a Drone's, and a Drone held open until the inbox is read is a Drone that
cannot finish the work the inbox is about.

**It reports a count and names the verb; it never pastes the entries.** That is
[`PLAN.md`](../../PLAN.md) §15.2's first rule arriving at the backstop: a hook that inlined every
unread body would put raw Drone output into Helm's window at the end of every single turn, which
is exactly how a context fills in three days. `fleet.inbox` is one tool call away.

**A dozen lines of `/bin/sh`, and no `jq`.** A hook that depends on a tool the machine may not
have is a backstop that silently stops backing anything up.

**The count comes from `armada fleet inbox --json`, not from a `grep` over the file.** The hook
reads `data.open` out of the envelope with one `sed`. It cannot decide the question itself: the
store is folded at read time, so being unread depends on the lines *after* a `raised` line, on
whether the entry carries a `job_uuid`, and on a repeated `uuid` being dropped — and a second
implementation of that fold, in shell, is how the hook came to spend the whole of M3 matching a
string no writer emits. `armada` is not a new dependency; the line above already runs the same
binary to sweep the fleet.

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

[`../fleet/inbox.md`](../fleet/inbox.md) · [`helm.md`](helm.md) · [`../fleet/answer.md`](../fleet/answer.md)
