# `armada helm enable` · `armada helm disable`

The switch that decides whether `armada helm` may become a session, on this machine. **It is the
whole authorization** — nothing downstream asks a second time.

> **Status: shipped.**

## Synopsis

```sh
armada helm enable [--json]
armada helm disable [--json]
```

## Arguments

Neither takes anything but the flags every verb shares — `--json` and `--color`.

## Why this exists

`armada helm` replaces this process with `claude`, which is the moment a real budget starts
spending against a real account. A verb that did that unconditionally could be reached by a
script, a shell alias, a test harness, or a mistyped line — and a fresh install must not be one
mistyped line away from opening a session nobody meant to open. So entering is gated by a switch,
and the switch is off until a person turns it on.

**One switch, and not two.** Entering used to be behind this *and* `--exec`, so a machine that had
already said yes still printed a command to paste. Asking twice does not make the first answer
more considered; it makes it look ignored. On, `armada helm` enters — and [`helm.md`](helm.md) is
where the two flags that opt out of that live.

## Why a machine fact and not a guild preference

Your guild syncs between every machine you use and describes **you** — voice, skills, what an
unattended Drone may do. `helm.enter` describes something else: whether a Claude Code session may
open **here**, right now. Those are different questions. A laptop you are sitting at answers one
way; a shared box, a CI runner, or a machine you configured once and have not opened a terminal on
since answers another. Putting this in the guild would mean flipping it once, anywhere, turns it
on on every machine that guild has ever been pulled onto — including ones nobody meant to grant it
on. So it lives in `~/.armada/machine.yml`, the file that never syncs, under its own `helm:`
section — the same shape `manifest:`'s section already takes there, one section per module.

## How it works

`enable` writes `helm.enter: true`; `disable` writes `helm.enter: false`. Both read first and
write only when the value actually changes, so running either twice in a row is silent about the
second time — the envelope's `changed` says whether anything was written.

Neither touches the guild, the persona, or any of the documents `armada helm` wires up. Being
*allowed* to open a session and being currently *able* to — a guild that exists, a persona that is
projected — are different questions, answered by different commands. Entering needs both to be
true.

**Off is the default**, which is also what a missing `machine.yml`, a missing `helm:` section, and
a section that fails to parse all mean. A fresh install cannot open a session until this runs, on
every one of those paths and not only the ordinary one.

## Output

```
OK  entering helm is on on this machine
```

`--json` carries `entering` (the value after this run) and `changed` (whether it moved).

## Dependencies

None. Either runs on a machine with no guild, no persona, and no `armada init` yet — the switch is
independent of everything `armada helm` itself needs.

## Exit codes

`0` written · `6` `environment` — `~/.armada/machine.yml` is not writable.

Full table and the one rule behind it: [`../reference.md`](../reference.md).

## See also

[`helm.md`](helm.md) — what the switch gates, and what `armada helm` refuses with when it is off.
