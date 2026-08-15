---
id: 018
title: A place for settings
status: RESERVED
module: cross-cutting
raised: real use, 2026-08-15
---

# A place for settings

**The ask.** *"I would imagine at some point we're going to need a global Armada configuration.
This is one example of something where the exec is either on or off, but I think there's gonna
be other things that we're gonna wanna be able to configure."*

He is right, and the evidence is that it has already happened twice without anyone deciding it
should.

## What exists, unplanned

`~/.armada/machine.yml` now carries two module sections — `manifest:` and, as of `helm.enter`,
`helm:`. Each arrived with its own verb pair and its own reader:

| Setting | Read by | Written by |
|---|---|---|
| `helm.enter` | `crates/helm/src/machine.rs` | `armada helm enable` / `disable` |
| Manifest's own keys | `crates/manifest/src/machine.rs` | `armada init`, and by hand |

**Two sections, two readers, two spellings of the same idea.** A third will make it a pattern
nobody chose. The collision that already bit — `machine.yml` written by Guild and parsed by
Manifest with `deny_unknown_fields` — is the same failure one turn earlier, and it was fixed by
namespacing per module rather than by giving the file an owner.

## The line this must not cross

`PLAN.md` §13.1 draws it and it is not negotiable: **what describes *you* syncs; what describes
*this machine and its running processes* never does.** `helm.enter` landed in `machine.yml` for
exactly that reason — putting it in the guild would silently enable Helm on every machine that
guild is ever pulled onto.

So a settings surface has to answer, per key, which side of that line it falls on. **A single
flat `armada config set <k> <v>` that cannot tell the two apart would be a way to leak a machine
fact into a synced repository**, which is the one mistake the split exists to prevent.

## What is actually open

- **Whether settings get a verb at all.** `armada helm enable` reads better than
  `armada config set helm.enter true`, and a purpose-named verb can say what it does in its own
  help. The counter-argument is that ten purpose-named verbs are ten things to discover, and
  `armada guild ls` and `armada manifest commands` exist precisely because listing beats
  remembering.
- **Reading versus writing.** Seeing every setting and its current value is useful immediately
  and is the cheap half; a generic writer is where the type and validation questions live.
- **Where a *portable* preference goes.** The guild is the synced half and already holds
  `settings.json`. If a setting is about the user rather than the machine, that is its home, and
  `armada guild ls` already shows it — which argues the guild half may need no new surface at
  all.
- **Precedence, once both halves exist.** If a key can be set in the guild and overridden on a
  machine, the rule for which wins has to be written down before the second such key ships, not
  after.
- **What `doctor` should say.** It already reports `helm argv` and the guild's state; a settings
  surface with no health check is one nobody notices has drifted.

**Not scheduled.** But the cost of leaving it is not zero: every module that adds a switch adds
its own verb and its own reader, and the third one is where retrofitting starts to hurt.
