# `armada helm`

The one agent you talk to. Typing `armada` with no arguments enters it; `armada helm` is the
explicit spelling.

> **Status: not built — M3.** ([`PHASES.md`](../PHASES.md) §8.5)

**Helm is a conversation, not a screen.** It is a Claude Code session, which is the whole design:
it needs no interface work, so it ships with Fleet instead of after everything else. The screen
is the [Bridge](bridge.md), and it is a separate thing you can run or not run — nothing below
Helm moves either way.

> **There is no `helm` binary.** Kubernetes owns that name and Armada runs on machines that have
> it. Helm is a subcommand and the bare-`armada` default, never a program on `PATH`
> ([`../glossary.md`](../glossary.md)).

## Synopsis

```sh
armada [--new] [--agent <name>] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `--new` | flag | off | Start a fresh Helm conversation instead of resuming. |
| `--agent <name>` | subagent name | `helm` | Use a different persona from `~/.armada/guild/subagents/`. |

## How it works

Assembles and execs one command:

```sh
claude --agent helm --mcp armada --resume <helm-session-uuid>
```

| Piece | Comes from |
|---|---|
| The persona | `~/.armada/guild/subagents/helm.md` — **yours**, editable, synced |
| The toolbelt | [`mcp.md`](mcp.md) — `fleet.*` and `manifest.*` |
| The conversation | A stable uuid in `~/.armada/`, so it is the same conversation each day |
| Awareness | [`inbox.md`](inbox.md) — a monitor plus a `Stop` hook |

Its job is **decompose → delegate → aggregate → report**. Classification is *not* its job; that
belongs to Fleet ([`../fleet/spawn.md`](../fleet/spawn.md)), because a Job must be
classifiable before Helm exists.

### Two structural rules

**Helm reads summaries, never raw transcripts.** Reading Drone transcripts fills its context in
about three days of work, after which it starts forgetting the fleet — the exact failure it exists
to prevent. This is a design constraint, not a tuning knob.

**Probe never interrupts a Drone.** `fleet.probe` summarises a transcript with a cheap model.
Messaging a busy agent to ask how it is going costs you the thing you were measuring.

## Output

A conversation.

```
> add rate limiting to the API, and figure out why the nightly job is flaky

  Two Jobs. rate-limit (feature) and nightly-flake (bug).
  I'll come back when either needs you.

> how's it going?

  rate-limit    — 14m, RUNNING. Checks green, writing tests. On track.
  nightly-flake —  9m, reproduced it, 3 of 5 runs. Needs you:
                   it wants to bump the CI timeout, which felt like your call.
```

`--json` prints the resolved launch command and exits without starting anything — for scripting
and debugging, not for use.

## Dependencies

| On | Why |
|---|---|
| `claude` | It is a Claude Code session. |
| An initialised guild | The persona lives there. |
| Fleet | Its tools are Fleet's verbs. |
| An MCP server registration | [`mcp.md`](mcp.md) |

## Exit codes

The process is replaced by `claude`, so the exit code is whatever it exits with.

`3` `bad_config` — no guild, or no Helm persona; run [`../init.md`](../init.md).

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`bridge.md`](bridge.md) · [`mcp.md`](mcp.md) · [`inbox.md`](inbox.md) · [`../fleet/spawn.md`](../fleet/spawn.md)
