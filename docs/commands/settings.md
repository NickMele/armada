# `armada settings`

List every setting Armada knows about: its current value, where it lives, and whether it syncs.
Read-only.

> **Status: built, read-only** — no writer. ([`docs/reserved/018-a-place-for-settings.md`](../reserved/018-a-place-for-settings.md))

## Synopsis

```sh
armada settings [--json]
```

## How it works

Three readers, reused rather than retyped:

1. **Manifest's section of `~/.armada/machine.yml`** — `crates/manifest/src/machine.rs`'s
   `MachineConfig::read`, the same reader every Manifest verb uses. Eight keys, each destructured
   by name, so a ninth key added to that struct and not added here fails to compile.
2. **Helm's section of the same file** — `crates/helm/src/machine.rs`'s `read`, the reader
   `armada helm enable`/`disable` already write through. Two keys: `enter`, and `mode` — the
   `--permission-mode` the session enters under. **No verb writes `mode`**, which is exactly why
   it is listed: this is the only place a reader finds out the key exists and what it is set to.
3. **The guild's config-shaped items** — `armada_guild::inventory::Inventory::items`, the same
   call `armada guild ls` makes. `settings.json`, `plugins.yml`, `mcp.yml` and `permissions.yml`
   are listed; a skill, a hook, a workflow or a memory fragment is content rather than a setting
   and does not appear here.

**A setting nobody has touched still gets a row**, carrying its documented default — `Manifest's`
reader already merges a missing file, or a missing key in one that exists, with the defaults.
"What can I configure" and "what have I configured" are the same question here, and omitting the
untouched ones would answer only the second.

## Output

**`STATUS · NAME · DETAIL`**, no `TIME` — nothing here is timed. `STATUS` carries which side of
the sync line a row is on rather than a health word, because that is the fact a reader most needs
and the one most easily got wrong:

```
  STATUS   NAME                      DETAIL

  MACHINE  manifest.cpu_slots        6 — ~/.armada/machine.yml
  MACHINE  manifest.port_block_size  10 — ~/.armada/machine.yml
  MACHINE  helm.enter                on — ~/.armada/machine.yml
  MACHINE  helm.mode                 auto — ~/.armada/machine.yml
  SYNCED   guild.settings.json       3 settings — ~/.armada/guild/settings.json

OK  5 settings
```

`MACHINE` describes this machine and its running processes and never syncs — `machine.yml`.
`SYNCED` describes you and travels with the guild between every machine you use. `helm.enter` is
`MACHINE` on purpose: pulling a guild onto a new laptop must not silently enable Helm there, and
`helm.mode` is `MACHINE` for the same reason — how much a session may do without stopping to ask
is a fact about the box you are sitting at.

`--json` carries `data.settings[]`, each row with `locality` (`"machine"` or `"synced"`), `name`,
`value` and `at` — the same four facts, unjoined.

## Dependencies

Reads `~/.armada/machine.yml` and the guild's config files. Writes nothing.

## Exit codes

`0` always, unless `~/.armada/machine.yml` exists and cannot be parsed, which is `6`
`environment` — the same failure `armada doctor` and every Manifest verb report for the same file.

Full table and the one rule behind it: [`reference.md`](reference.md).

## See also

[`doctor.md`](doctor.md) · [`guild/ls.md`](guild/ls.md) · [`helm/helm.md`](helm/helm.md)
