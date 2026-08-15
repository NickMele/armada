# `armada helm`

The one agent you talk to.

> **Status: built — M3.** The launch is assembled, the inbox is wired and the conversation is
> remembered. **Entering the session is behind `--exec`**, which is the one thing here that
> spends.

**Helm is a conversation, not a screen.** It is a Claude Code session, which is the whole
design: it needs no interface work, so it ships with Fleet instead of after everything else. The
screen is the [Bridge](bridge.md), and it is a separate thing you can run or not run — nothing
below Helm moves either way.

> **There is no `helm` binary.** Kubernetes owns that name and Armada runs on machines that have
> it. Helm is a subcommand, never a program on `PATH`
> ([`../glossary.md`](../../glossary.md)).

## Synopsis

```sh
armada helm [--agent <name>] [--new] [--exec] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `--exec` | flag | **off** | Become the session. **This is the one that spends.** |
| `--new` | flag | off | Start a fresh Helm conversation instead of resuming. |
| `--agent <name>` | subagent name | `helm` | Use a different persona from `~/.armada/guild/subagents/`. |

## Assembling is free; entering is not

`armada helm` writes the configuration, reports the command, and **starts nothing**.

| | Costs |
|---|---|
| Assembling the command | nothing |
| Entering the session | a real budget, against a real account, for as long as it is open |

A verb that opened a Claude Code session as a side effect of being run can be reached by a
script, by a shell alias, by a test harness and by a mistyped line, and each of those spends. So
the spend is behind **two** conditions and needs both: `--exec` on the line, and a terminal on
the other end. A `--exec` into a pipe is refused rather than degraded — a caller who typed it
asked for a conversation, and answering with the report they would have got anyway looks like
Armada ignoring the flag.

> **Bare `armada` does not enter Helm.** [`PLAN.md`](../../PLAN.md) §15.1 gives it the bare word
> eventually and that remains the intended end state; it is deliberately not wired, because the
> bare word is the most typeable thing on the machine and the failure mode of getting it wrong
> is a session nobody meant to open. `armada` alone is the orientation page.

## How it works

Assembles one command, and prints it:

```sh
claude --agent helm \
       --mcp-config ~/.armada/helm/mcp.json \
       --plugin-dir ~/.armada/helm/plugin \
       --settings   ~/.armada/helm/settings.json \
       --session-id <uuid>          # the first launch, which mints it
```

`--resume <uuid>` replaces `--session-id <uuid>` on every launch after the first, which is the
whole of *"the same conversation each day"*. The two are not interchangeable: `--resume` against
a uuid Claude Code has never seen fails with *no conversation found*, and `--session-id` against
one it has seen is a second conversation wearing the first one's name.

| Piece | Comes from |
|---|---|
| The persona | `~/.armada/guild/subagents/helm.md` — **yours**, editable, synced. Seeded from [`templates/guild/subagents/helm.md`](../../../templates/guild/subagents/helm.md); the four behaviours it fixes and why are in [`PLAN.md`](../../PLAN.md) §15.4. |
| The toolbelt | [`mcp.md`](mcp.md) — `fleet.*` and `manifest.*`, registered as `--mcp-config` |
| The conversation | `~/.armada/helm/session.json` — a uuid, the persona it belongs to, and whether it has run |
| Awareness | [`inbox.md`](inbox.md) — a monitor plus a `Stop` hook, both written to `~/.armada/helm/` |

Its job is **decompose → delegate → aggregate → report**. Classification is *not* its job; that
belongs to Fleet ([`../fleet/spawn.md`](../fleet/spawn.md)), because a Job must be
classifiable before Helm exists.

### What it writes

Everything lands under `~/.armada/helm/`, which never syncs — every file in it names a path on
*this* machine ([`PLAN.md`](../../PLAN.md) §13.1).

| File | What it is |
|---|---|
| `mcp.json` | the toolbelt, as a `--mcp-config` document |
| `plugin/` | a session-scoped plugin carrying one monitor |
| `settings.json` | the `Stop` hook, registered for this session and not for the machine |
| `stop-inbox.sh` | what that hook runs |
| `session.json` | the conversation |

**Rewritten on every launch rather than written once**, because each names a path — the `armada`
binary, the inbox — and a machine whose home directory moved would otherwise keep a registration
pointing at a binary that is not there. A file already holding exactly those bytes reports
`unchanged`, so a reader who edited one by hand can see that it was replaced.

### Two structural rules

**Helm reads summaries, never raw transcripts.** Reading Drone transcripts fills its context in
about three days of work, after which it starts forgetting the fleet — the exact failure it exists
to prevent. This is a design constraint, not a tuning knob. It reaches the `Stop` hook too: the
backstop reports *how many* entries are unread and names `fleet.inbox`, rather than pasting the
bodies into Helm's window at the end of every turn.

**Probe never interrupts a Drone.** `fleet.probe` summarises a transcript with a cheap model.
Messaging a busy agent to ask how it is going costs you the thing you were measuring.

## Output

Four rows and the command.

```
  STATUS     WIRED         DETAIL
  WRITTEN    toolbelt      ~/.armada/helm/mcp.json
  WRITTEN    monitor       ~/.armada/helm/plugin
  WRITTEN    backstop      ~/.armada/helm/stop-inbox.sh
  UNCHANGED  conversation  ~/.armada/helm/session.json

  enter with claude --agent helm --mcp-config …

OK  helm · conversation new · nothing started; `armada helm --exec` enters it
```

The last line is load-bearing. Four `WRITTEN` rows and a launch command, without it, read
exactly like a Helm that is now running.

`--json` carries the same facts plus `argv` as a vector — a `$HOME` with a space in it is
ordinary on macOS, and a consumer that had to split the printed line back into words would break
on exactly those machines.

## Dependencies

| On | Why |
|---|---|
| `claude` | It is a Claude Code session. |
| An initialised guild | The persona lives there. |
| A projected guild | `--agent` names a persona Claude Code has to be able to find, and `~/.armada/guild/` is not on its load path until [`../guild/project.md`](../guild/project.md) puts it there. |
| Fleet | Its tools are Fleet's verbs. |
| An MCP server registration | [`mcp.md`](mcp.md) |

## Exit codes

Without `--exec` the process is Armada's and exits normally. With it, the process is replaced by
`claude`, so the exit code is whatever it exits with.

`2` `bad_invocation` — `--exec` where there is no terminal.

`3` `bad_config` — no guild, no such persona, or a persona that is not on Claude Code's load
path. The three are reported separately, because three different commands fix them.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`bridge.md`](bridge.md) · [`mcp.md`](mcp.md) · [`inbox.md`](inbox.md) · [`../fleet/spawn.md`](../fleet/spawn.md)
