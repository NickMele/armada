# Spike 5 — What does a real Job cost, and can quota be read?

**Cost: about $0.10 on Sonnet for a small feature Job, and the figure is arithmetically exact.**
**Quota: the percentage exists, but not on the channel a headless Drone can read.** The NDJSON
stream reports *whether you are allowed* and *when the window resets*, and no quantity. The number
itself — `five_hour` and `seven_day` utilization — is live in Claude Code on every request and is
handed to the **statusline**, which does not run in `-p` mode. See *Where the number actually lives*
below: this is a plumbing problem, not an absent measurement.

Measured 2026-08-23 against Claude Code **2.1.241**, `claude-sonnet-5`, on a Max subscription.

## The Job

A `no_std`-free Rust crate with an unimplemented `parse` and **six failing tests** covering sections,
comments, quoted values, last-key-wins and a line-numbered error. The Drone had to implement it and
run `cargo test` until green. Same task, same starting commit, three times — the repo was reset with
`git checkout -- . && git clean -fd` between runs. **All three ended 6/6 green.**
Runner: [`005-run.sh`](005-run.sh).

## What it cost

| Run | Turns | Wall | Cost | Fresh input | Output | Cache read | Cache write |
|---|---|---|---|---|---|---|---|
| 1 | 7 | 22.0s | **$0.146473** | 12 | 1,225 | 241,836 | 21,458 |
| 2 | 7 | 24.0s | **$0.087344** | 12 | 1,544 | 259,180 | 5,011 |
| 3 | 4 | 20.5s | **$0.063366** | 8 | 1,159 | 171,198 | 4,380 |

Mean **$0.099**. The same Job, done correctly three times, varied **2.31×** in price.

**Almost nothing about that spread is the work.** Output tokens moved by 33% across the three runs;
cost moved by 131%. What moved was cache: run 1 wrote 21,458 cache tokens at 2× the input rate and
paid 58.6% of its bill for the privilege, while run 3 wrote 4,380 and paid 27.6%. Fresh input tokens
were **8 to 12** in every run — 99.99% of the input was cache reads or cache writes.

## Can the dollar figure be trusted?

**The arithmetic: yes, exactly.** `total_cost_usd` reconstructs from `usage` to floating-point noise
(residual ≤ 1.7e-16) using published Sonnet 5 rates. Solving the two unknown cache rates from runs 2
and 3 and checking against run 1 recovers them exactly:

| Component | Rate per 1M | Source |
|---|---|---|
| Input | $2.00 | Sonnet 5 introductory rate, through 2026-08-31 |
| Output | $10.00 | same |
| Cache read | **$0.20** | solved — 0.10× input |
| Cache write (1h) | **$4.00** | solved — 2.00× input |

So the figure is not a guess and Armada can recompute or re-price it from `usage` alone.

**What it means: no.** `claude auth status` reports `authMethod: "claude.ai"`,
`subscriptionType: "max"`, and spike 3 recorded `apiKeySource: "none"`. **Nothing here is billed per
token.** `total_cost_usd` is what the run *would* have cost at API list price. That is the likely
explanation for Phase 0's note that v1's dollar figures read as inflated in real use: they were not
wrong, they were denominated in a currency this account does not spend.

## Can quota be read?

**Fifteen `rate_limit_event`s were captured across every run of spikes 3, 4 and 5. All fifteen are
byte-identical:**

```json
{"status":"allowed","rateLimitType":"five_hour","resetsAt":1787488200,
 "overageStatus":"rejected","overageDisabledReason":"org_level_disabled","isUsingOverage":false}
```

Six fields. **None of them is a quantity.** There is no utilization, no percentage, no token count,
no remaining budget — and only one window (`five_hour`) ever appeared.

**This is not a regression, and v1 already knew.** `v1-final`'s
`crates/core/src/fleet/drone.rs` documents the same behaviour from the other side:

> *"`None` is the ordinary case and not a failure. The field only rides along once a threshold is
> crossed — the `allowed_warning` shape — so a window well inside its limits reports its reset and no
> percentage."*

v1 also recorded seeing a `seven_day` kind alongside `five_hour`, and a Job dying at 03:55 on
`status: "rejected"`. So both windows and the utilization figure are real. **They arrive late.**

**Not reproduced here, and honestly so:** I never observed `allowed_warning`, `rejected`, or
`seven_day` — reproducing them means actually approaching the account's limit, which this spike did
not do. The measurement establishes what is available in the ordinary case, not what the warning
shape contains.

## What that costs the design

The step's own fallback is the answer: **budget gating becomes a post-hoc counter rather than a
gate.**

| Mechanism | Verdict |
|---|---|
| Quota-percent floor gate ("don't dispatch below 20% remaining") | **Buildable, but not from the Drone's stream.** The number is live on every request; reaching it needs one of the three routes below |
| Hard stop on exhaustion | **Works.** `status` leaving `allowed`, and `resetsAt` for when to resume. Binary, not proportional |
| Budget gating in dollars | Works as a counter over `total_cost_usd`, on a notional figure that no invoice will match |
| Fan-out cap | Derivable from headroom **only once a route to the percentage is chosen**; otherwise from concurrency or wall-clock |
| Judge call-count ceiling | Unaffected — it counts calls, not quota |

## Where the number actually lives

The first version of this write-up concluded the percentage was unavailable. That was wrong — it
concluded from one channel. Corrected 2026-08-23, same day, by inspecting Claude Code 2.1.241 itself.

**The service sends it on every response.** The binary carries a family of
`anthropic-ratelimit-unified-*` headers — `-status`, `-reset`, `-overage-utilization`,
`-slow-budget-utilization`, and the per-window `…-<window>-utilization` that v1's archived
`drone.rs` comment names.

**Claude Code keeps it in process and gives it to the statusline.** The payload builder reads an
internal store and emits:

```
rate_limits: {
  five_hour:  { used_percentage: utilization * 100, resets_at },
  seven_day:  { used_percentage: utilization * 100, resets_at },
}
```

That is exactly what this machine's own statusline reads —
`.rate_limits.five_hour.used_percentage` and `.rate_limits.seven_day.used_percentage`. Both windows,
a real percentage, continuously, with no warning threshold involved.

**But the statusline does not run headless.** Tested directly: a `-p` run with
`--settings '{"statusLine":{"type":"command","command":"…"}}'` pointing at a script that dumps stdin
to a file. The run exited 0 and **the script never fired**. So a Drone cannot be asked to report its
own quota by this route.

### Three routes, and what is known about each

| Route | Status |
|---|---|
| Read `rate_limit_event` from the Drone's stream | **Measured. Insufficient** — carries status, window kind, reset and overage flags; no quantity until a warning threshold |
| Harvest the statusline payload from a session Fleet runs for that purpose | **Plausible, untested.** The payload has both windows. Needs a session type where the statusline fires — `-p` does not |
| Put a proxy in front of the Drone (`ANTHROPIC_BASE_URL`) and read the headers | **Untested, and the honest unknown is auth** — whether subscription OAuth traffic survives a redirected base URL. The binary also ships a `claude gateway` subcommand for enterprise auth/telemetry, which is the same idea supported |

**None of these is measured end to end, so none is a design decision yet.** What changed is the
question: it is no longer *"can the number be known"* — it can, continuously, both windows — but
*"which pipe does Armada read it from."*

## The finding the step did not ask for

**A per-Job dollar budget mostly measures cache warmth.** Three identical, equally successful runs of
the same Job priced at $0.063, $0.087 and $0.146. A budget set at $0.10 would have failed run 1 and
passed the other two for reasons that have nothing to do with the work, the code, or the Drone. Any
budget denominated in dollars needs a margin wide enough to swallow a 2.3× swing — at which point it
is not really a budget. **Turns and wall-clock were far steadier** (7/7/4 turns, 20.5–24.0s) and are
the better ceiling for catching a Drone that is going wrong.

## Rough scale, for planning

At mean $0.099 per small feature Job on Sonnet, and Opus 5 at 2.5× Sonnet's list rates, a hundred
such Jobs is roughly $10 on Sonnet or $25 on Opus of *notional* spend — and zero actual spend on a
Max subscription until the five-hour window says otherwise. The number that will stop a fleet is the
window, and the only warning it gives is late.
