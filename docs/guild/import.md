# `armada guild import`

Restore a guild from a bundle.

> **Status: not built — M2.**

## Synopsis

```sh
armada guild import <path> [--merge] [--force] [--json]
```

## Arguments

| Flag | Type | Default | Meaning |
|---|---|---|---|
| `<path>` | file path | — | The bundle to import. Required. |
| `--merge` | flag | off | Merge into the existing guild instead of replacing it. Conflicting files are reported and skipped, never overwritten. |
| `--force` | flag | off | Replace an existing guild. Refuses without it. |

## How it works

1. **Validates the bundle** before touching anything — structure, then every schema-backed file.
   A bundle that fails validation changes nothing.
2. Unpacks to `~/.armada/guild/`, replacing or merging per the flags.
3. **Initialises git** and makes one initial commit. A bundle carries no history
   ([`export.md`](export.md)), so the imported guild starts a fresh one.
4. **Projects** it: registers plugins, writes managed memory regions, applies settings.

`machine.yml` in a bundle is **ignored unless it is absent locally**, so importing your own
export onto a different machine cannot overwrite that machine's paths and capacity with
another's.

## Output

```
validated  ok
imported   19 skills · 12 hooks · 4 workflows · 3 fragments
skipped    machine.yml (this machine has its own)
projected  ✓
```

`--json` returns one result per imported category plus the projection outcome.

## Dependencies

`git`. No network. An existing guild only matters for `--merge` / `--force`.

## Exit codes

`0` imported · `2` `bad_invocation` — a guild exists and neither `--merge` nor `--force` was given · `3` `bad_config` — the bundle failed validation, and **nothing changed**.

Full table and the one rule behind it: [`reference.md`](../reference.md).

## See also

[`export.md`](export.md) · [`init.md`](init.md)
