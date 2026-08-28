# Spike 9 — How long does a step take, and what does it do while it takes it?

**A step's own clock is minutes, not half hours: median 131s, p90 437s, and the longest honest one
1777s across the 31 completed steps this repository has run.** So `wall_clock` at 1800s was a
ceiling nothing reached, and it is now 1500s.

**The more useful result is the second one, and it is a negative.** Every step that was actually
stuck was *quiet*, not long. Inside a step doing honest work the longest silence between two Drone
events was 79s; the worst stuck step sat silent for 1636s and was killed by a person two seconds
before the old ceiling would have fired. Elapsed time does not separate a slow step from a stopped
one. Time since the last event does, and no tripwire reads it.

Measured 2026-08-28, over Jobs run 2026-08-26 to 2026-08-28.

## What it was measured over

| | |
|---|---|
| Repository | this one, and only this one |
| Jobs | 16 — six completed, three failed their gate, five killed by a person, two escalated |
| Workflows | `bug` and `feature` |
| Steps | `plan`, `scope`, `implement`, `verify`, `tests`, `handoff` |
| Model | `claude-sonnet-5` throughout |
| Work | documentation edits and small Rust and TypeScript changes |
| Checks | `build`, `test`, `diff_nonempty`, `evidence_scope`, against a warm Cargo cache |

**None of that generalises.** A cold `cargo build --workspace` on this machine is minutes and none
of these steps paid for one; a repository whose Checks are slow will see a different distribution
of the same measurement, which is why the numbers in `crates/armada/src/serve.rs` stay marked
provisional.

## Where the numbers come from

Two sources, and the first is the one that matters:

- **The store's `job_events`.** A `step_transition` row carries the step id and the instant, so
  `not_started -> running` to `running -> advanced` is the same interval `StepNorms::wall_clock` is
  compared against — Fleet's own Checks, its gate, and any retry all sit inside it, because a retry
  does not restart the step's clock.
- **`.armada/transcripts/*.jsonl`.** Every Drone event with the instant Fleet saw it. Rows written
  before the step label existed carry no step, so calls are attributed to a step by falling inside
  its interval, which is the same subtraction `Working::calls_this_step` does.

[`009-step-spans.py`](009-step-spans.py) reproduces every table below from those two.

## How long a step takes

| | n | median | p90 | p95 | max |
|---|---|---|---|---|---|
| **All completed steps** | 31 | 131s | 437s | 790s | 1777s |
| `plan` | 9 | 220s | 448s | | 492s |
| `implement` | 7 | 107s | 1363s | | 1777s |
| `verify` | 6 | 126s | 147s | | 153s |
| `handoff` | 6 | 20s | 55s | | 59s |
| `scope` | 3 | 298s | 356s | | 371s |

Nine steps in ten finish inside 500s. The two beyond it — 1087s and 1777s, both `implement` — were
honest work, and both had already passed the call norm, so the wall clock buys no look on them that
the call count had not already bought.

## What a ceiling would have cost

| ceiling | completed steps over it | of those, already over the call norm |
|---|---|---|
| 600s | 2 | 2 |
| 900s | 2 | 2 |
| 1200s | 1 | 1 |
| **1500s** | **1** | **1** |
| 1800s | 0 | — |

Every candidate costs the same, which is to say nothing, because no measured step was slow without
also being talkative. The ceiling was chosen from the other end instead: a step's clock holds
Fleet's Checks and does not restart on a retry, so an honest step can spend
`PROVISIONAL_CHECK_BUDGET` on one Check plus a p90 step's own work — 1337s — without anything being
wrong. 1500s is the first round number above that floor and below the longest honest step observed.

**A trip spends the step's only look.** `fleet::converging` marks a step `Looked` after one Judge
call whichever wire fired, so a lower ceiling does not buy earlier detection; it spends the
attention a later, real thrash would need.

## How many calls a step makes

The call norm is compared per step, so it is measured per step. The number in the code was derived
from turns per invocation, which is a different and smaller unit.

| | n | median | p90 | p95 | max |
|---|---|---|---|---|---|
| Tool calls per completed step | 31 | 18 | 68 | 137 | 361 |

At 60 the norm sits just under the p90: four of the 31 steps would have bought a look. It was left
where it is. Sixty is the more sensitive of the readings available and a sample this size is not a
reason to make a tripwire fire less.

**The `calls / turns` ratio in `fleet::Progress::calls` does not hold across this data.** Most of a
Drone's tool calls arrive after the harness has reported a result and before it announces the next
invocation: of the 435 calls in the longest Job's transcript, 318 fall outside a
`started`-to-`ended` pair. Delimit an invocation by consecutive `ended` events instead and the
ratio runs from 0.93 to 6.69 for invocations past fifteen turns, rather than the 0.92 to 0.99 that
comment claims. The claim was an artefact of dropping the calls that fell outside the pair. It does
not change what the code does — Fleet counts calls and never reads `turns` — but it does mean turns
cannot be used to sanity-check a call norm.

## The steps that never finished

Eight steps went `running` and never advanced, because a person killed the Job or it failed. They
are the interesting half of the data, and they are why the tier needs a signal it does not have.

| step | ran for at least | tool calls | longest silence | ended as |
|---|---|---|---|---|
| `scope` | 1798s | 33 | **1636s** | killed by a person |
| `implement` | 796s | 157 | 13s | completed_failed |
| `plan` | 413s | 0 | **409s** | killed by a person |
| `tests` | 294s | 19 | 46s | killed by a person |
| `implement` | 287s | 36 | 33s | completed_failed |
| `plan` | 167s | 4 | **147s** | killed by a person |
| `implement` | 40s | 3 | 13s | completed_failed |
| `implement` | 4s | 0 | 4s | escalated |

The first row is the whole argument. Its Drone worked for 162s, said "Scope submitted", ended its
invocation with a refusal recorded, and then said nothing for 27 minutes while the step stayed
`running`. The process was alive, so nothing about liveness would have noticed; the call count was
33, so the call norm never came close; and 1800s of wall clock arrived two seconds after a person
gave up and killed it.

## The signal that separates them

| across the 31 completed steps | median | p90 | max |
|---|---|---|---|
| Longest silence between two Drone events inside a step | 15s | 46s | **79s** |
| Silence between the last Drone event and the step ending | 0s | 16s | 19s |

An honest step is never quiet for longer than about eighty seconds. Three of the eight stuck
steps were quiet for longer than any honest step by a factor of two or more. **A silence threshold
anywhere between two and five minutes separates every case in this data**, which no elapsed-time
ceiling does at any value.

That tripwire does not exist. It is `stalled`'s — `escalation-triggers.toml` describes a
heartbeat/liveness timer with a poke and then an escalation, and `crates/config/settings.toml`
records that Fleet owns it — and nothing in `crates/fleet` implements one. Until it does, a Drone
that stops speaking is found by a person.

## What this changed

| | before | after |
|---|---|---|
| `wall_clock` | 1800s, unmeasured | 1500s, floored on the Check budget and capped by the observed maximum |
| `calls` | 60, from turns per invocation | 60, restated against calls per step |
| `report_grace` | 120s, from spike 4 | unchanged |

## What this does not answer

- **Whether any of it holds on another repository.** Everything here is one repository, one model,
  a warm cache and small changes.
- **What a step costs when a Check is slow.** No measured step paid for a cold build, and the
  ceiling's floor is an argument from `PROVISIONAL_CHECK_BUDGET` rather than an observation.
- **What the silence threshold should be.** The separation is stark enough that a number between
  two and five minutes is obviously safe here, and picking one is the job of whoever builds the
  tripwire against more than eight stuck steps.
