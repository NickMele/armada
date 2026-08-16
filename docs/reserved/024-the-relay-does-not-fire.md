---
id: 024
title: The relay does not fire under `--print`
status: BUG
module: fleet
raised: a Job driven end to end, 2026-08-16
---

# 024 — The relay does not fire under `--print`

**The claim [`020`](020-the-tui-decided.md) §1 rests on is false for a Drone.** That section
decided the Drone's `Stop` hook ticks the Job, and argued it from
[`PLAN.md`](../PLAN.md) §15.3 — *"hooks are the spine — an agent can forget to report progress,
but it cannot forget to stop."* It can, if nothing asks it to.

## The measurement

A `design` Job was spawned and watched from `explore` through `hand-over` on 2026-08-16. Every
piece of the relay is present and correct:

| Piece | State |
|---|---|
| `~/.armada/jobs/<uuid>.settings.json` | written, registers a `Stop` hook |
| `~/.armada/jobs/<uuid>.stop.sh` | written, mode `0755`, waits for its own process group then runs `armada fleet tick` |
| `--settings <path>` in the Drone argv | present |

And the hook never ran. The Drone's own transcript records hook events, so the question is
answerable rather than inferred:

```text
system: hook_started   SessionStart:startup
system: hook_response  SessionStart:startup
system: init
…
result: success  is_error False
```

**`SessionStart` fired; `Stop` never appears.** `~/.armada/recent.jsonl` agrees — across the
whole run the only `fleet tick` is the one typed by hand.

Two things follow, and the second is the sharper one:

- **`SessionStart` came from the operator's own `~/.claude/settings.json`**, not from Armada's
  `--settings` document, which defines only `Stop`. So the file Armada passes is not obviously
  being merged the way the design assumed.
- **Nothing advances a Job on its own.** Every Job reaching a step boundary sits at `SILENT`
  until a person runs `armada fleet tick`. The backstop sweep exists and is correct, and nothing
  calls it either.

## What is not yet known

**Which of two causes this is has not been established, and the fix differs.** Either
`--settings` hooks are not applied to a `--print` session at all, or `Stop` is not an event a
`--print` session emits — it ends rather than stopping. The first is a wiring bug; the second
means the whole mechanism needs a different event, and `020` §1's *"the exchange ending **is**
the event"* is the sentence that would be wrong.

Answering it costs one probe session and no guesswork: register a `Stop` hook that touches a
file, run `claude --print` with it, and look for the file.

## Why it was invisible until now

Every earlier Job died before it could show this. They stalled on a posture that denied their
tools ([`011`](011-what-a-drone-may-do-unattended.md)), then on an MCP server that was never
attached, then on a gate that could not match its own artifact pattern. A Job had to get far
enough to *finish an exchange cleanly* before the missing tick was the thing you noticed.

## Also worth recording

`armada fleet answer` takes the entry's **uuid**, and `fleet inbox --json` spells that field
`uuid` while the human table's column header is `ID`. A caller reading the JSON for an `id` finds
nothing. Small, and it cost a wrong answer to a stale entry while driving the Job above — which
then let the gate ask again and burned two more exchanges.
