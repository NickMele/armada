# Journey 4 — Monitor Active Work

**What it is:** "What's currently running?" — a live scan, not an action.

Design fidelity: Full on job detail, which is drawn whole. Active jobs stays the M1 subset — one list, a Job's current step updating live, and a terminal state with its reason — until the Job Board settles the row it shares.

---

**Trigger:** Curiosity check-in, or confirming a Job you just dispatched actually started.

**Concepts touched:** Bridge (Active Jobs), Drone.

**Milestone:** Board. Active jobs shipped under M1 — Dogfood and is history here; job detail is the open work.

**Design file:** `Journey 4 - Monitor active work.dc.html` in the Armada Mockups project. One file per journey; this document is the record, the file is the drawing. Active jobs and job detail are both drawn there.

**Milestone design file:** `M1 - Dogfood.dc.html`, blocks `1a`, `1b`, `1d` and `1e`. Added 2026-08-23. What M1 needs and no more: the shell with one surface in the rail, the empty list in both Fleet states, the list at five job states, and a running job's rail. Three decisions came out of it — Fleet's three status-bar strings, an ungated step saying so in words, and the pulse narrowing to one mark per screen.

Analysis: Complete. UI/UX design: In progress.

## Flow

Open Bridge → Active Jobs → lightweight heartbeat per active Drone: status, elapsed time, current step. No turn-level detail (diff, tool calls, full history) at this level — that requires deliberately opening Debug on a specific Drone.

## Job detail

**The screen is one arrangement at every state.** The run is a tree on the left, the selected step fills the panel, and the step's story reads in the order it happened. Why: the screen had an arrangement per state, and below the header no region sat in the same place twice.

### The run

**A step is a row in a tree, and its facts open beneath it.** Each step carries what it produced, what it cleared and how long it took.

**The chevron opens a step's facts; the row selects it.** Why: the tree holds short facts and the panel holds long content, so the two never compete.

**Elapsed is a figure, never a chart.** Why: a filled bar reads as progress, and a step has no percentage.

**An attempt is a row, not a counter.** Why: attempts beside each other show whether a Drone is trying different things or rephrasing one, and a count shows neither.

**A path keeps its basename.** The directory truncates and recedes; the filename does not truncate at any width.

### The panel

**The story is Drone instructions, then Activity log, then Produced.** Each is a card with its own header.

**Opening one chapter collapses the others to their header line.** Why: the order stays readable while one part of it is long.

**A chapter whose content has no end opens as a trailing sheet instead.** The activity log runs to 1676 entries on a real Job and the diff is the Job's whole patch across several files — neither is a longer version of something the panel can hold, so the panel stops trying. The header line stays where it was and is the way back. `Esc` closes it, which is what `Esc` already means; what the map gains is the opening, `Enter` on detail, where nothing was bound before.

**One sheet at a time.** Opening the diff while the log is open replaces it, and `Esc` returns to the panel rather than to the previous sheet. Why: a layer that pops back to another layer makes one key mean two depths of *back*.

**This supersedes the inline expander for those two chapters, which is built.** `StepStory` opens all three in place today, and `apps/desktop/src/renderer/src/JobDetail.tsx` holds the open chapter for the step. Drone instructions keeps that treatment and the collapse rule above; the other two leave the panel. `#286` builds the sheets.

**The Job's brief sits above the step, on the panel's raised surface.** Why: every step is read against it.

### Produced states line counts on a Job that has stopped, and not on one that is running

A running Job's chapter reads `3 files · all inside the plan`. A finished one reads `3 files · +94 −31 · all inside the plan`. The difference is a measurement, not a field somebody left out.

**The drawing shows a running Job carrying counts, and is wrong on that point.** Counting a file is the same xdiff that renders its patch, and every route to it costs the same — the call that skips the per-file work answers totals only. Measured against this repository, release build:

| Footprint | The file list | The counts |
|---|---|---|
| 6 files, 105 lines | 0.08µs | 1.3ms |
| 104 files, 7.7k lines | 0.25µs | 25ms |
| 414 files, 59k lines | 1.0µs | 90ms |

The live reading is taken every two seconds while somebody is watching, inside a Fleet turn of 250ms, so a large footprint would spend a third of that turn on a number nobody can read moving that fast. `crates/fleet/src/footprint.rs` takes it once instead, on the transition that ends the Job, and writes it down — which is also the only reading that survives `armada clean` giving the worktree back.

**A file with no counts is one nothing could count**, not one that changed nothing. A binary file has no patch to count and a file moved without being edited is a real zero, so the pair is absent rather than zeroed and the chapter draws no churn where it is missing.

### The phase strip

**A step's phases and its gate tiers are one progression** — instructed, working, submitted, then the step's Checks, its Judge, and you.

**Each stage is a control.** Opening one states what that stage is, what it is waiting on, and where it stands.

**An absent tier is not a failed tier.** A step declaring no Check and no Judge draws what does advance it.

### Checks and the Judge

Two tiers, and drawing them as one row of chips risks reading as one kind of thing.

| | Checks | Judge |
|---|---|---|
| What it is | Commands the repository declares | A model reading the work |
| Who runs it | Fleet, never the Drone | Fleet |
| Judged against | An exit code | The step's acceptance criteria |
| What it may do | Pass or fail | Refuse only |

### The activity log

**One stream carries the Drone's turns, Armada's injected turns and Fleet's own events.** Every entry names who.

**Every entry opens to its payload.** A command opens to its full text, its output, its exit code and where it ran.

**An opened log holds its position and does not follow the tail.** Why: a person opening 1676 entries is looking for one of them, and a followed tail pulls it out from under them. *Jump to now* carries the count of what arrived, and the held strip states where the reading is.

### Acts

**An act that changes a step sits in the panel header. An act that ends or replaces the Job sits in the Job header.** Redirect, restart step, override the verdict and re-run the gate act on a step; kill the Job, redispatch and approve act on the Job.

**The Job header carries one split button.** Its lead is the act the state calls for, its divided segment is the caret and nothing else, and every other Job-level act sits behind it — so the control's width stops depending on how many acts the state offers, and the title keeps the row. Three side-by-side buttons collapsed the title's column and wrapped a seven-word title to three lines.

**The fill is the state, not the act.** Accent where the Job is waiting on a person — escalated, awaiting review, evidence disputed. Secondary where it is not, which is a running Job and a finished one. Same height in both cases.

**The lead is never destructive.** Kill sits in the menu on every state that offers it, because the lead segment is what a stray `Enter` hits. It keeps `x`.

| State | Lead |
| --- | --- |
| Running | Pilot |
| Escalated | Pilot, accent |
| Awaiting review | Review, accent |
| Evidence disputed | Redispatch as a new job, accent |
| Killed, failed | Redispatch as a new job |

**Within the panel header, the step acts take the accent.** Why: the object of attention on this screen is the open step. The Job header's fill is decided by state, above, and the two headers do not compete.

**A step's help text is a tooltip carrying its binding**, shown on hover and on focus. What a destructive act costs is stated again in its confirmation.

### Failure never reads as one thing

| State | Treatment | What it means |
|---|---|---|
| Waiting on you | Amber, no surface | Everything mechanical cleared |
| Stopped | `--step-stopped-bg`, `flag` in `--fg-default` | Retries spent, a person decides |
| Failed | Hued surface and hued glyph | The Job is over |

### A failed Check is repaired, not fatal

**A failed mechanical Check hands the work back to the Drone that produced it**, with the Check's output, under the step's retry limit. Why: a failing test is work, and the Drone that wrote the code is what should fix it.

**This contradicts [Job](../concepts/job.md)**, which states that a failed mechanical Check ends the Job at `completed_failed`, terminal and out of reach. Both cannot be true.

**Where a step lands once retries are spent is unresolved** — `[retries-exhausted-destination]` on `crates/core-model/domain/README.md`. The screen assumes the Job holds with its Drone alive and idle, because redirect costs no respawn and consumes no attempt from there. Fleet does not do that yet: a spent budget ends the Job at `completed_failed`, and `crates/fleet/src/tests/retrying.rs` asserts it. The drawing is ahead of the code here on purpose, and the error-states drawing marks the badge provisional rather than naming a status that does not exist.

**A Drone proposes what to try before it holds.** Picking one drafts the instruction, which stays editable, and writing one from nothing is always available.

## Design Notes

- Heartbeat is intentionally lightweight — Bridge stays a scanning surface, not a firehose.
- **Active jobs wireframed 2026-08-21** as block `4h`. Both halves of the trigger are answered by ordering rather than a control: rows that need a person sort above rows that do not, and within each group oldest first, because the thing waiting longest is the thing most likely to have gone wrong. The header states the count, how many need you, and how many are waiting on the Job Board.
- **One line of step state per row, not a rail.** A rail answers *why is this stuck*, which is job detail's question. The row says which step, how far through, and whether it is moving. No new component — the same 60px stacked job row the Job Board uses, with the step added to the field run.
- **Job detail wireframed 2026-08-21**, out of design order. The rail's gate row and two open colour questions could only be read on this screen, and both were blocking. Drawn on a Bug job escalated at `regression_verify` with the suite green — the Check passed and the Judge refused, which is the escalation shape worth designing for.
- **The escalation payload is prose, not bars.** Briefing register: a headline naming the step, the field run, three attempt rows each citing what the refusal was grounded in, and the two artifacts named by role — the diff as work product, `root_cause` as the yardstick it is judged against. The reading a person takes from the column is that the drone keeps trimming the diff toward a note it will not amend, so the offered action is rewriting the brief rather than retrying.
- Debug is reachable from here on any active Drone, not just ones that have escalated — same read-only, trigger-adaptive view described in Respond to a Push Alert, minus the trigger-specific framing (thrashing/evidence-suspect) since nothing has fired yet.

## Settled here

- **A step's hue and the Job badge may disagree, and neither is authoritative.** An advanced step stays `--step-advanced` whatever the Job did afterwards. The rail answers where the work got to; the badge answers what the Job is doing now. Making a step's colour conditional on its Job's state was drawn and rejected — it reads better on one screen and makes the same step render two ways for reasons that have nothing to do with the step. **One stated exception:** in the Active Jobs **step bar**, advanced segments are neutral (`--border-strong`) and hue marks the current step alone. A bar compresses a whole rail into 72px, so five rows of six advanced steps is thirty green marks and the hue stops meaning anything — the same scaling argument that took the accent off list rows and the pulse off every running Job. The rail itself is unchanged, because it shows one Job and has room for the reading.
- **Progress on a list row is a step bar, not a fraction.** "4 of 7" has to be read and converted; a bar is read at a glance, and read down a column it shows which Jobs are near the end. One segment per step, so segment width itself says how long the workflow is. Past `--border-strong`, remaining `--border-subtle`, current segment in its step-activity hue. The exact count moves to the tooltip.
- **The Judge panel row.** A panel passes only on unanimity, so its verdict is binary whatever its size — five Judges are one veto with five chances to fire, and a row per Judge renders four met rows that changed nothing. The panel is one row carrying its size and count; what stacks beneath it is refusals, each with its citation and its Judge as a trailing key. A panel that passed has no rows beneath it. Judge identity is metadata, not a heading: identical models on identical inputs, so "judge 2" is a citation key and not an author. A single Judge is the same row with the size and key omitted, so the rail gains no component.
- **A stopped step is a sixth activity value.** Retries spent is not retrying and not waiting on you — folding it into `waiting` would render a designed human gate and a dead stop alike. It takes a `flag` glyph in `--fg-default` on a `--step-stopped-bg` row. The surface exists because a glyph only holds while its row is selected: **background states what the row is, the accent left edge states which row you are on**, so the surface is constant and selection adds the edge. One per rail. `octagon-x` and a bare attempt count were both drawn and rejected. Recorded in `tokens/status.css` and the design system contract, with `flag` reserved.
- **Hard prerequisite is a lock at the step row's trailing edge**, in `--fg-muted`, label only. A hard prerequisite is a property of the workflow definition rather than of this run — it reads the same on a Job that has not started as on one that stopped — so it takes the quietest treatment that survives being repeated three times on one rail. Prose does not: a sentence per step turns a seven-row rail into fourteen rows of hedging. Needs a `lock` row in Iconography, reserved to this — not a permission, not a security state, not a locked file.
- **An escalated Job usually holds a live Drone, idle.** The liveness clock suspends on entry to `escalated`, so where a step stopped mid-work — which is where escalation usually happens — the Drone keeps its PID, worktree and session while it waits. It is gone where the escalation reason is `interrupted`, and gone at a step boundary, because a Drone belongs to a workflow step and there is no process between one step and the next. Pilot's third trigger path keys on `assigned_drone` being null rather than on the status, so it stays correct for any Drone-less state — **and a surface that keys on the same bit to choose between Redirect and Restart Step does not.** `concepts/job.md` has the rule: the act is decided by where the Job stands. What is spent on an escalated Job is autonomous progress, not the process.
- **The escalation action set is Kill, Rephrase and redispatch, and Pilot**, with Pilot the primary and the view's only accent fill. On an escalated Job autonomous progress is spent by definition, so the act that moves it forward is a person taking it; Pilot opens the confirmation modal where Take Over, Assist and Restart Step are chosen, and the header does not preview them. Kill is ghost and leftmost. **`Override` was invented and is retired** — it named waiving a gate, which no page sanctions: Evidence gates are identical for human and Drone work, and Pilot is a change of operator, not of standard. This also settles the lock as a label with no row action behind it — the way past a locked step is Pilot.

  (`Rephrase and redispatch` was itself later retired in favour of **Kill & Redispatch** — see Respond to a Push Alert.)

  **Where these acts sit is superseded by Job detail above**: an act on a step moved to the panel header and took the accent with it, so the Job header carries only the acts that end or replace the Job. What survives here is the reasoning — `Override` stays retired, and the way past a locked step is Pilot.

- **The origin tag takes plain words and no chip.** "Dispatched by you" / "Found by Fleet" / "Drafted in Helm," replacing "Manual" / "Auto-detected" / "Helm-drafted" — settings vocabulary naming modes the lexicon does not have. **Amended 2026-08-21: origin loses the chip entirely.** It was a bordered pill the same height, radius and padding as the status badge beside it, separated only by colour, and the rule that the outlined one is never a status appeared nowhere on screen. **A chip is a status.** Origin is plain sans text in `--fg-muted`, which finishes the sentence the contract had already started — not a Job state, so not a hue, and not a chip either. Three treatments now do three jobs: chip for the state, sans for who and what, mono for machine values. **The leading dot is mandatory on a status badge** and forbidden elsewhere, fixing an inconsistency the kit already had. The cost is real — origin gets quieter, and it was already the thing people miss; if that becomes a problem the fix is giving it somewhere with room, not a shape that lies about what it is. Moving origin into an attribution sentence under the title was drawn and rejected earlier: it reads better on a detail screen and does not fit a list row.
- **A list row never takes an accent fill, and carries exactly one control.** Settled 2026-08-21 while drawing Active Jobs. Every row has one secondary split button whose label is the act its state calls for — `Pilot` on an escalated Job, `Approve` on one waiting for you, `Open` on one that is simply running — with the alternatives behind the caret. Clicking the row does the same as Open; the button exists so the column reads consistently and the action is always named rather than guessed from the row being clickable. No ellipsis: it names nothing, so a person has to open it to learn whether it holds Kill or Redirect. None takes a fill, because fourteen awaiting-review Jobs would be fourteen accent blocks — the same failure as the pulse. Urgency is carried by the badge and the ordering; the accent is spent on job detail, where the object of attention is a single Job.
- **The Job's diff is the Job's, and a step only chooses where it opens.** Fleet commits once at the end, so there is no per-step patch to show. The sheet header names the branch rather than the step, and the file rail carries the only step-scoped fact a diff has — which step wrote each file.

## Settled since — Redirect, Kill, Pause and Judge scope

Two questions this journey once carried open are now decided, and both are recorded in full on Respond to a Push Alert since that is where the ladder itself lives:

- **Redirect, Kill and Pause are all available on a healthy, non-escalated Drone**, decided 21 Aug 2026, with a redirect recorded on the Job as mid-step context injection rather than a step restart.
- **Whether a Judge call is a sub-dispatch that consumes the fan-out cap** — decided no. A Judge call is a call with a verdict: no worktree, no branch, no rail, no Job Board row, no `dispatched_by`, under a cent. A sub-dispatch is a Drone spawning a Job with its own workflow, worktree, rail and Board row. Folding Judge calls into the cap would make "2 of 5" meaningless, since Bug's `fix` step alone sets `panel_size: 3` and would burn 3 of a 5 fan-out budget at one step — volume that is entirely by design.
- **Whether the Judge sees the Drone's transcript** — decided no, for every Judge call including the mid-step thrashing pass. The thrashing check now works from mechanical signals (turn count vs step norm, wall-clock vs ceiling, declared-plan-vs-live-diff) plus the declared scope and the work product so far, never the transcript or self-report. The rationale is keeping bias out: a detector that reads the Drone's own narration of why it is struggling is being argued at by the thing it is judging, and the contamination is worst exactly when the Drone is off-rails. What the Judge receives when the evidence is not a diff is now a per-`evidence_type` table on the Workflow concept page rather than a general principle, because the differences are not derivable from one rule.
- **Whether there is an aggregate attempt bound on a looping step**, given `retry_count` resets each iteration — decided yes: an aggregate attempt cap on `default_gate_policy`, overridable per step, with unbounded a legal value, and the default set at today's effective product so nothing changes behaviour by surprise. The cap is a runaway guard for the case nobody is watching (an away-from-desk batch grinding overnight), not a safety limit for the case the engineer is watching and can just kill the Job.
- **How a loop workflow's appended prior-iteration context stays under `max_context_size`** — decided: the most recent work product only, plus every prior verdict. Work products (drafts, documents, mockups) are large, so only the most recent is appended; verdicts are a sentence or two, so all of them are kept, because five rounds of feedback showing the same note went unaddressed three times is exactly the pattern `iteration_cap` exists to catch.
- **Whether "same failure" means same step and same reason, or same step and any reason** — decided same step, any reason. A step that stalls and then fails a gate reads as "second time at step 3," not as two first-time failures, because the suggested action — "the last redirect did not change the outcome" — is true either way.

## Findings carried from the design pass

- **A workflow step has no human name.** Found by drawing Active Jobs, where rows read `regression_verify` in mono. That is honest — it is machine-derived — but it is a schema identifier on a surface a person scans, and it tells them nothing. `step_id` is the only name a step has in the Bug sample, and the enum→verb table covers Job states rather than workflow steps, so there is no map to look it up in. See Open questions below.
- **Whether the rail shows the label, the `step_id`, or both** — **settled 2026-08-21: the label, everywhere.** Step names are labels in sans on every surface, including the rail on job detail. Nouns naming the artifact: *Reproduction*, *Root cause*, *Fix*, *Regression check*. A step is a unit of work with a name, so it reads as one; the gate rows beneath it keep mono identifiers, because a Check is a command. **Sans names work, mono names machinery.** The `step_id` survives only where a machine reference is genuinely the point — in citations, and in prose that names the schema explicitly.
- **The running pulse does not scale to a list** — **settled 2026-08-21: the pulse is scoped to the current step of the focused Job**, one on screen at a time, wherever the rail or the list appears. It was scoped per rail, where one row is current by definition; a list has one per running Job, and at fourteen it is what the motion tokens forbid outright.
- **The pulse moves with the reading, not with the rail.** Still one running mark per screen, now on the thing being read. With a sheet open the tree's current step is behind the layer, so the sheet's live mark takes the pulse and the tree's stops — and it stops entirely once the Job is no longer running.
- **The stopped surface competes with the focused row.** A stopped Job carries `--step-stopped-bg` and the focused row carries `--bg-hover` with the accent bar. Two tinted rows in one list saying different things. It holds at one stopped Job because the accent edge does the work; three would read as three focus rows.
- **A Convoy has no single workspace**, so its row names its first write target and counts the rest. Settled on the Board's row, which this list reuses — see [Job Board](../concepts/job-board.md).
- **Redirect, Kill and Pause on a healthy, non-escalated Drone** — settled, see above.
- **The origin tag reads as a status** — resolved 2026-08-21, see Settled here above.
- **The copy-on-click rule claims more than it means.** `docs/contracts/design-system.md` says anything in mono copies on click and names a path and a command among its examples. A path that opens a file cannot also copy it, so the rule wants narrowing to identifiers — a job id, a branch, a drone id.
- **A workflow definition carries no version and no timestamp.** The panel can say a Job runs the workflow as it stood when the Job was dispatched, which needs nothing new, but not whether the definition has moved on since. `[workflowdef-schema-gaps]` on `crates/core-model/domain/README.md` already names `version` as a key the samples use and the catalogue does not.
- **`evidence_type` has no visual value, and none of its values is a capture.** A Job asked to change a screen proves it with a diff, which is the least useful thing on that screen.

## Open questions

- **[evidence-mcp-submission-schema]** What is the Evidence MCP tool's submission schema?
  The Evidence MCP tool is the one sanctioned way a Drone reports step completion. Its full submission schema is not yet specified. Partly settled: the freeform prose a Drone writes stays deliberately freeform — advisory only, never gates anything — and is explicitly markdown, with Bridge rendering it as formatted markdown a stated Phase 1 UI requirement. Also decided, 22 Aug 2026, is the subset M1 implements: a submission is a signal plus an account, and everything that gates is computed on Fleet's side — self-report is a signal, never the source of truth. The M1 tool input carries `evidence_type` (enum, must equal the current step's declared type), `claimed` (what the work now does, as an observable — refused when empty), `shown_by` (the artifact demonstrating it — refused when empty, since a field naming no artifact evidences nothing), `not_claimed` (the gap and the side effect — required and legitimately empty, so it is a string and not an option: empty is an answer, absent is not), and — until 26 Aug 2026 — `note`. **`note` is gone**: a Drone that submitted one on a step whose type was not `facts_note` was refused, spent its last turn on the refusal and left the Job unfinished, having broken a rule it was never told. It is the same three fields on every step now, because there is no work a note did that they do not: what you did is `claimed`, where to look is `shown_by`, what you left is `not_claimed`. A step whose product is a written finding names the file it wrote in `shown_by`. The consequence, which is open: **`evidence_type` no longer changes anything a Drone does** — requiring or refusing the note was its only behaviour, and what is left is a comparison against the step's own declared type that Fleet fills in and so cannot fail. Settled 26 Aug 2026: the earlier single `summary` field is retired in favour of the three, which are the field names the Agent Copy Contract defines and the Drone prompts already ask for by name. No step id — Fleet knows the current step. No `source` field, and its absence is the mechanism: a Drone must not be able to mark evidence as human-attested. No file lists, no attachments, no `context_paths`. The call returns `recorded` and nothing more; the Drone cannot infer advancement from a successful submission. Deferred beyond M1: `context_paths`, `declare_plan_at`, evidence scope objects, gaming baselines, clarification rounds, and the typed payloads for `bundle`, `document`, `review_findings` and `failing_test`. **What remains open is the schema beyond this M1 subset.**

## Related

Respond to a Push Alert for the escalated version of this same Debug view.
