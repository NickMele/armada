# `armada`

The front door. Every module, what state it is in, and what to type next.

> **Status: built.** The layout is frozen by `tests/golden/render/menu.plain` and its `.tty`
> twin, and by `menu-fresh` beside them — the same screen on a machine where nothing is set up
> yet.

**Bare `armada` used to be the `--help` root page under the wordmark, and before that it was
promised to Helm.** [`../reserved/020-the-tui-decided.md`](../reserved/020-the-tui-decided.md)'s
menu decision took it from both. The argument is short: entering Helm is off by default on a
machine (`helm.enter`), so a bare word that *usually refuses* is a worse front door than one
that lists what is there — and a list of verb *names* is not what a person opening a terminal in
an unfamiliar directory needs. They need to know where they are.

[`../PLAN.md`](../PLAN.md) §15.1 keeps the older argument, marked superseded, because it is what
this decision was weighed against.

## Synopsis

```sh
armada [--json]
```

`armada --help` is unchanged: it is still the list of every verb Armada owns, and this screen's
last line says so.

## How it works

**Five rows, one per module, Helm first because it is who you talk to.** Each is read from that
module's own verb — nothing here is a second source:

| Row | Read from | `READY` when | otherwise |
|---|---|---|---|
| `helm` | `helm.enter` in `~/.armada/machine.yml` | the switch is on | `DOWN` — [`helm/helm.md`](helm/helm.md) |
| `fleet` | [`fleet/ls.md`](fleet/ls.md) | — | `WAITING` if something needs you, `RUNNING` if work is in flight, else `OK` |
| `inbox` | [`fleet/inbox.md`](fleet/inbox.md) | — | `WAITING` while anything is open, else `OK` |
| `manifest` | workspace discovery | this directory resolves to a workspace | `DOWN` — nothing claims it |
| `guild` | `~/.armada/guild` | there is one | `DOWN` — [`guild/init.md`](guild/init.md) |

**It runs before workspace resolution**, like the Fleet and Helm verbs and for the same reason:
most directories a person opens a terminal in are not workspaces, and refusing in all of them
would make this the screen you cannot reach exactly when you do not know where you are. Manifest
says so on its own row instead.

**Manifest's row asks *where am I*, not *what is up*.** It resolves the workspace and stops;
[`manifest/status.md`](manifest/status.md) is what probes docker, and paying for that on a screen
opened to orient yourself would be the wrong trade.

**No new status words.** Every word is one [`../glossary.md`](../glossary.md) already fixes.
`DOWN` is Manifest's own word for *not standing up*, which is what an off switch, an unclaimed
directory and an absent guild each are.

## Output

```
  STATUS   MODULE    DETAIL                             VERB
  READY    helm      resumes your conversation          armada helm
  WAITING  fleet     4 jobs · 2 need you · 1 stalled    armada fleet ls
  WAITING  inbox     2 questions waiting on you         armada fleet inbox
  READY    manifest  armada.yml — this workspace        armada manifest status
  READY    guild     19 skills · 2 hooks · 4 workflows  armada guild ls

  `armada --help` lists every verb · `armada <module>` opens one
```

**The wordmark is above this on a terminal and never in a pipe** — six lines of block characters
at the top of what an agent reads is noise it has to learn to skip
([`render.md`](render.md)). This is the one screen that draws it: bare `armada` is the moment of
orientation, and `--help` is the page you reached for in a hurry.

**`DETAIL` is a fact and `VERB` is the instruction, and they are kept apart on purpose.** A fact
that also carried its command — *"no guild yet — `armada init` creates one"* — grows until the
flexible column truncates it, and a truncated command is not a shorter answer, it is the wrong
one ([`fleet/board.md`](fleet/board.md) makes the same argument about a resume line).

**`VERB` varies with the row's state**, which is what makes the row usable rather than
decorative. On a machine where nothing is set up:

```
  STATUS  MODULE    DETAIL               VERB
  DOWN    helm      off on this machine  armada helm enable
  OK      fleet     no jobs              armada fleet ls
  OK      inbox     nothing open         armada fleet inbox
  DOWN    manifest  no armada.yml here   armada manifest init
  DOWN    guild     no guild yet         armada init
```

A column that always said `armada helm` would, beside `DOWN`, be advertising the one command
that refuses. The Bridge's `p` key — *pause* over a running Job, *resume* over a paused one —
already worked this way.

**There is no summary line, and the absence is load-bearing.** A word over the five would be
derived from the worst row and would describe no module in particular — the same objection `020`
raises against an aggregate status over several Jobs. It is also the one field that would have to
be computed from two modules at once, which is the boundary
[`../ARCHITECTURE.md`](../ARCHITECTURE.md) §1.9 draws: this screen may **read** every module and
must never become where they read **each other**.

`--json` returns `results[]` with `module`, `status`, `fact` and `verb` per row, and nothing
beside it.

## Dependencies

None. It reports on what is missing rather than requiring it.

## Exit codes

`0` — always, when the screen could be drawn. The words describing the modules are on the rows;
the envelope's own status is the **command's**, and it is `OK` even with every row reporting
`DOWN`. That is the read-verb rule ([`../PLAN.md`](../PLAN.md) §3.1): the exit code describes the
query, not the thing queried.

## See also

[`helm/helm.md`](helm/helm.md) · [`helm/bridge.md`](helm/bridge.md) ·
[`fleet/ls.md`](fleet/ls.md) · [`reference.md`](reference.md)
