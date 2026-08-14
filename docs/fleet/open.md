# `armada fleet open`

Take over a session yourself.

> **Status: not built — M3.**

**Armada does not own a terminal.** A worker is an ordinary resumable Claude session in a git
worktree, so cmux, the Claude app, or a plain shell opens it. This command tells you how; it
does not build a multiplexer, and deliberately so ([`PHASES.md`](../PHASES.md) §9.1 F1).

## Synopsis

```sh
armada fleet open <session> [--print] [--exec] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `<session>` | session name | — | Which session. Required. |
| `--print` | flag | **default** | Print the worktree path and resume command. |
| `--exec` | flag | off | Change directory and exec `claude --resume` directly, replacing this process. |

## How it works

Looks up the session's worktree and uuid and produces the two facts needed to enter it. With
`--exec` it performs the `cd` and `exec` itself.

**It does not stop the session first.** If a turn is in flight, resuming interactively while it
runs is a conflict — check [`ls.md`](ls.md) for `waiting` or `idle` before taking over, or use
[`answer.md`](answer.md) if all you have is a decision.

## Output

```
worktree  ~/.armada/workspaces/api/rate-limit
resume    claude --resume 15bfa340-33b1-4f81-bd7f-688f0f01dbb0
```

`--json` returns `worktree`, `uuid`, `branch` and the assembled command.

## Dependencies

An existing session. `--exec` additionally needs `claude` on `PATH`.

## Exit codes

`0` printed · `2` `bad_invocation` — unknown session.

With `--exec` the process is replaced, so the exit code becomes `claude`'s.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`answer.md`](answer.md) · [`ls.md`](ls.md)
