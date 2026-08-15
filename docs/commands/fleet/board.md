# `armada fleet board`

Board a Job — take it over yourself.

> **Status: built — M3.**

**Armada does not own a terminal.** A Job's conversation is an ordinary resumable Claude Code
session in a git worktree, so cmux, the Claude app, or a plain shell enters it. This command
tells you how; it does not build a multiplexer, and deliberately so
([`PHASES.md`](../../PHASES.md) §9.1 F1).

> **Board does not mean "attach".** The nautical sense is *step aboard and take the wheel*, not
> *watch over someone's shoulder*. Boarding a Job whose Drone is mid-turn does not stream that
> Drone's output to you — it hands you the conversation so you can drive it yourself. Attaching
> to a live process is the pty work withdrawn in §9.1 F1, and nothing here reintroduces it.

## Synopsis

```sh
armada fleet board <job> [--print] [--exec] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `<job>` | Job name | — | Which Job. Required. |
| `--print` | flag | **default** | Print the worktree path and resume command. |
| `--exec` | flag | off | Change directory and exec `claude --resume` directly, replacing this process. |

## How it works

Looks up the Job's worktree and uuid and produces the two facts needed to enter it. With
`--exec` it performs the `cd` and `exec` itself.

**It does not stop a running Drone first.** If a turn is in flight, resuming interactively while
it runs is a conflict — check [`ls.md`](ls.md) for `PAUSED` or `BLOCKED` before boarding, or use
[`answer.md`](answer.md) if all you have is a decision.

**Boarding a Job with no live Drone is the ordinary case**, not a recovery path. A Job persists
without a process ([`PLAN.md`](../../PLAN.md) §14.1), so most boarding happens after a Drone has
finished its turn and exited.

## Output

```
  STATUS    DETAIL
  worktree  ~/.armada/workspaces/api/rate-limit
  resume    claude --resume 15bfa340-33b1-4f81-bd7f-688f0f01dbb0

OK  rate-limit, branch armada/rate-limit
```

**The DETAIL column is fixed here and flexible on every other verb.** A truncated resume
command is not a shorter answer, it is the wrong one — and this whole verb exists to be
pasted.

`--json` returns `job`, `worktree`, `uuid`, `branch` and the assembled command.

## Dependencies

An existing Job. `--exec` additionally needs `claude` on `PATH`.

## Exit codes

`0` printed · `2` `bad_invocation` — unknown Job.

With `--exec` the process is replaced, so the exit code becomes `claude`'s.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`answer.md`](answer.md) · [`ls.md`](ls.md) · [`../helm/bridge.md`](../helm/bridge.md)
