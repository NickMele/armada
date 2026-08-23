# Spike 4 — Can Fleet inject a turn into a live session?

**Answer: yes, and the Drone obeys it — but delivery waits for the current turn to end.**
A message pushed into a running session lands at the next turn boundary, not immediately. If the
Drone is inside a tool call, the injected turn waits for that call to return. **A Drone stuck inside
a hung command cannot be poked**, which is exactly the case the poke exists to catch.

Measured 2026-08-23 against Claude Code **2.1.241**, three runs.

## The mechanism

`--input-format stream-json` (documented as "realtime streaming input") with stdin held open.
Fleet writes one JSON object per line:

```json
{"type":"user","message":{"role":"user","content":"STOP. Change of instructions: …"}}
```

`--replay-user-messages` re-emits the message on stdout **at the moment it is consumed**, which is
what makes the latency below measurable rather than inferred — it is an acknowledgement, not an echo
of the write. Driver used: [`004-driver.py`](004-driver.py).

## What was measured

| Run | Session state when injected | Delivered after | First action after | Transcript |
|---|---|---|---|---|
| 1 | mid-task, between two fast tool calls | **1.59s** | 3.70s | [`004-transcript-mid-task.ndjson`](004-transcript-mid-task.ndjson) |
| 2 | idle but alive — previous turn had ended | **2.85s** | 2.85s | [`004-transcript-idle-session.ndjson`](004-transcript-idle-session.ndjson) |
| 3 | inside a 40s foreground `Bash` call | **33.14s** | 36.27s | [`004-transcript-during-tool-call.ndjson`](004-transcript-during-tool-call.ndjson) |

Run 3's 33.14s is not latency. It is the remainder of the tool call: the command started at t=5.6s,
the injection went in at t=13.6s, and the message was consumed at t=46.8s — 3µs after the tool
result arrived. **Delivery is a turn-boundary event.** It costs whatever is left of the current step.

## Does the Drone actually obey it?

Yes, and it abandoned its own instructions to do so. Run 1's Drone was told to create ten files one
at a time; three files in, it was told to stop and instead record how many it had made. It stopped
at `step03.txt`, wrote `INTERRUPTED.txt` containing `3`, and reported *"Stopped per your instruction.
3 stepNN.txt files existed (step01–step03)."* The injected turn wins over the standing task, and the
Drone can see what it had already done.

## The finding the step did not ask for

**`result` is a turn boundary, not the end of the session.** In run 2 the session emitted
`result/success` at t=10.4s, then took the injected message, emitted a *second* `system/init` at
t=12.3s, did the work, and emitted a *second* `result/success` at t=16.3s — all in one process, one
session id. A Fleet that treats the first `result` as "the Drone has exited" will reap a live
session; a Fleet that treats `init` as "a Drone has started" will double-count.

This compounds spike 3's finding. `result` is neither a verdict nor a lifetime.

## Liveness during a long tool call

Not silent. Run 3's 40-second foreground command emitted `system/task_started` at t=9.7s and a
`tool_progress` event at t=35.7s, followed by `system/task_notification` and the `tool_result`. So a
Drone inside a long command is distinguishable from a Drone that has genuinely stopped — the
distinction fixture 1 needs — without any injection at all.

## What this costs the six mechanisms

| Mechanism | Verdict |
|---|---|
| Evidence clarification round | Works. The Drone is between turns by definition when it has just submitted evidence |
| The gate's own outcome message | Works, same reason — decided 22 Aug 2026, and the decision holds |
| `redirect_drone` | Works, at the cost of the current step's remaining tool call |
| Pilot handoff | Works |
| **The poke** | **Compromised.** A Drone silent because it is hung inside a tool call cannot be reached; the poke is delivered if and when the call returns. `tool_progress` tells you which kind of silence you have, so the poke can at least be aimed correctly |
| **Force-interrupt directive** | **Unresolved.** Injection cannot interrupt; it queues |

## The lead this leaves open, unmeasured

`init.capabilities` advertises `interrupt_receipt_v1` and `interrupt_cancel_queued_v1`, which implies
a control channel distinct from the user-message stream measured here. Nothing in `--help` documents
its wire format and guessing it would be inventing rather than measuring, so it is recorded as a
lead: **whether a running tool call can be interrupted is a separate question from whether a turn can
be injected, and this spike answers only the second.**

## Consequence for the design

The gate's outcome-as-injected-turn design survives — the Drone is idle at exactly the moment the
gate speaks. The kill-and-respawn fallback is not needed. What does not survive is the assumption
that the poke reaches a Drone in any state: it reaches an idle Drone and a Drone between tool calls,
and it queues behind whatever the Drone is currently running.

## A note on the transcripts

**The captures are byte-for-byte except in one place.** The `init` event's
inventory of the operator's own tooling — connected MCP servers, plugins,
skills, subagents, and the tool list naming them all again — is replaced by a
count, and the home path by `user`. This repository is public and that inventory
is personal; the count is what the findings above actually rest on, so nothing
they claim has been weakened. Every other event is exactly as it arrived.
