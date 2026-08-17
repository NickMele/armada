---
id: 024
title: The relay's watcher never ticks
status: FIXED
module: fleet
raised: a Job driven end to end, 2026-08-16
---

# 024 — The relay's watcher never ticks

**The claim [`020`](020-the-tui-decided.md)'s first decision rests on is false for a Drone.** That section
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

  **Answered 2026-08-17 by [`034`](034-the-job-daemon-lands-the-work.md): a daemon calls it.** Two
  facts narrowed this first, and both are worth keeping. Every Drone's `Stop` hook already sweeps the
  *whole* fleet rather than its own Job — which is why `armada_fleet::pass` takes a machine-wide lock
  — so while anything at all is alive the fleet is being swept. The unrecoverable state is therefore
  a fleet where **nothing** is alive: no hook will fire again and it stays frozen until a person
  types a verb. And a Job is not a process — it is a record in `~/.armada/jobs/` — so it cannot
  notice that time passed on its own account. The watcher is always some other process, and `034`
  names it.

## What the hook does and does not explain — corrected 2026-08-16

**A `Stop` hook does fire under `--print`.** Measured directly, with a minimal probe rather than
by inference:

```sh
claude --settings ./settings.json --permission-mode dontAsk \
       --allowedTools Read --print "say banana"
# -> STOP HOOK FIRED 06:41:52
```

**So the conclusion first recorded here was wrong, and the reasoning is worth keeping because it
is a trap.** It rested on the Drone's transcript showing `SessionStart` and no `Stop` event — but
the transcript is closed before a `Stop` hook runs, so a `Stop` event can never appear in it.
Absence there is not evidence of anything.

What still stands is the `recent.jsonl` evidence: across a whole run the only `armada fleet tick`
is the one typed by hand. The hook fires and the tick does not happen, so **the fault is inside
`stop.sh`**, between the two.

The script forks a watcher that reads its own process group, polls until the group leader is gone,
and only then ticks. The likeliest failure is that the watcher is inside the Drone's own process
group and is killed with it — `drone::stop` signals the group, and a watcher waiting for that
group's death is a watcher the death takes with it. **Not yet proved**, and it is provable without
spending a token: fork the same watcher under a process group, kill the group, and see whether the
tick lands.

**A second reading of the probe above is worth noting.** The first attempt failed with *"Input
must be provided either through stdin or as a prompt argument"* because `--allowedTools Read "say
banana"` let the variadic flag eat the prompt — the exact trap `drone.rs` documents and guards
with an invariant test. The guard works; a hand-written probe has no such guard.

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

## Fixed 2026-08-16

`leader=$PPID`. Claude Code runs a hook in its own process group, so `$$`'s group named the hook
and the watcher waited for itself — it broke at `waited=0` and ticked while the Drone was still
running, and `tick` correctly reported `WORKING` and moved nothing. `$PPID` is the Drone.

**The test that was supposed to catch this asserted `ps -o pgid= -p $$`**, so it pinned the bug in
place while reading as proof of the opposite. It is `AGENTS.md`'s own rule arriving as a defect:
asserting on a string proves you built the string you meant, not that it works. The assertion now
requires `$PPID` and forbids the process-group form.

Verified on a real Job: spawned at `explore`, it advanced to `articulate` with no manual tick, and
`recent.jsonl` recorded six hook-driven ticks over the run.
