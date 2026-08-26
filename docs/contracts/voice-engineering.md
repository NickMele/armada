# Voice Contract — Engineering Requirements

**Kind:** spec. **Governs:** engineering requirements behind the Voice
contract — the enum→verb map, lint enforcement, and rename sweeps.

Read by whoever builds the copy artifacts.

Carries copy decisions from the [Design System — UI &
Voice](design-system.md) contract that impose requirements on Fleet, the
schema, or CI. Each arose as a consequence of a voice decision rather than
as a feature.

**Intended fate:** these should be absorbed into the Phased Implementation
Plan and this document retired. It exists so they are not lost in the gap
between a design document and a build plan.

---

## Requirements

**1. Escalation event payload must carry decision facts.**

Step index, last-touched path, poke count, elapsed. The briefing register
(P2) is only renderable if these are on the event. Without them the copy
degrades to a log line no matter how it is written.

*Source: P2. Phase: with the escalation event shape.*

**2. Verification results must carry a source field, and events must
carry an actor field.**

Two fields, not one. They were conflated in the first draft of the
contract.

- **Verification source:** closed vocabulary of three — `Check`, `Judge`,
  `Attestation`, the last meaning a person attested and settable only by a
  human. The P4 hedging device. Answers how far to trust a result. A
  record carrying `Attestation` must render distinctly from one verified
  mechanically, and its criterion verdicts read `confirmed` / `withheld`
  rather than either mechanical vocabulary.
- **Actor:** `human`, `Helm`, `Drone`, `Fleet`. Audit attribution. Answers
  who did this, and is what the three-way separation (human / Helm /
  Drone) depends on at the point a person actually reads it.

Events may carry one, both or neither. A manual change during Pilot is
actor=human with no verification source. An allowlist denial is
actor=Fleet with no verification source. A failed gate is verification
source=Check with actor=Drone.

*Source: P4 and the Debug/Pilot audit trail. Phase: with the verification
result type, and with the Pilot takeover work.*

**2b. Spend figures must carry their source class and the active billing
mode.**

The universal row shows quota % in personal-machine mode
(provider-reported, measured) and dollars in work-machine mode (derived,
marked approximate until validated). v1's $ figures were often inflated
or inaccurate, and the same number gates dispatch, so the payload needs
to distinguish a measured value from an estimated one rather than
leaving the UI to guess.

*Source: P4 estimated class. Phase: 0 validation, then 3a where budget
gating lands.*

**3. Fleet emits discrete fields, not rendered strings.**

Bridge, the push alert and Helm each render the same event differently. A
pre-rendered string gives the push truncation rule nothing to truncate
intelligently.

*Source: status grammar. Phase: Ground Zero, since it shapes the ipc
types.*

**4. Recurrence counter keyed on lineage.**

Keyed on `redispatched_from` lineage, not job id, because kill-and-redispatch
creates a new job and a second stall would otherwise look like a first one
every time.

**Built now, unused until the behaviour decision.** Lineage that was not
recorded cannot be reconstructed later, so deferring the field means the
deferred behaviour arrives with no history to act on.

*Source: repetition rule. Phase: Ground Zero schema.*

**5. Enum-to-verb table as a single artifact.**

One file owning the map, plus a test asserting every enum variant has
exactly one entry. Every vocabulary, not only Job states: Job states and
their reasons, step verdicts, and criterion verdicts on each verification
source. A new variant in any of them must not be able to ship with no
copy. Same enforcement shape as `store` being the sole deserializer, and
the same codegen intent already noted for the status tokens.

*Source: status grammar. Phase: Ground Zero, alongside the ipc codegen.*

**6. Copy lint in two places.**

- CI, over the Bridge static string catalog. Catches drift in hand-written
  UI copy.
- The checks runner, over agent-written PR bodies, commit messages and
  Judge summaries. See [the Agent Copy Contract](agent-copy.md).

*Source: P6 and the Agent Copy Contract. Phase: CI lint early, checks-runner
lint with the checks runner.*

---

## Open questions

- **Judge rename sweep.** — **Done Aug 2026.** Judge is decided and the
  workspace is consistent. Fixed: Fleet ("Mechanical Check → Judge
  Check"), Job (six instances across the ownership split, status enum,
  Facts/Evidence and workflow_status), Drone (sub-dispatch paragraph),
  Workflow Design System (two grammatical artifacts left by an earlier
  partial sweep — "an Judge"). Already correct and untouched: the Phased
  Implementation Plan (including the Phase 0 step 18 latency spike,
  already "Judge call latency"), [testkit Fixture Specs](testkit-fixtures.md),
  [Configuration](configuration.md) & [Adapters](adapters.md). Deliberately left alone:
  the **Original Concept Notes** document, which is a historical record
  of the source material rather than live documentation, and the [Design
  System](design-system.md) lexicon, where "the auditor" is listed as a
  banned alternate and is supposed to appear.

- **[verdict-artifact-rows]** Do the step-verdict and criterion-verdict
  vocabularies get written into the enum→verb artifact as their own rows?
  Three vocabularies are decided — step verdict pass reads "Step N of M
  verified"; Check criterion verdicts read `passed` / `failed` / `not
  reached`; Judge criterion verdicts read `no objection` / `refused` — but
  none exists in the artifact yet. Requirement 5 above specifies one
  artifact with one test covering every vocabulary, not only Job states,
  so this is that requirement's remaining gap. Left undone, designers
  hand-write these headlines and the copy rules get broken by omission,
  which is how "Judge passed" happened.

Also bearing on this document, and written where each belongs: `[status-bar-loudness]` in `design-system.md`. A question has one home — answering it in two places is how one of them goes stale.
