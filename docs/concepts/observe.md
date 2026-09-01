# Observe

**What it is:** Watching a Drone work while it keeps working — a read-only view of its turns, taking nothing over and changing nothing about the Job.

---

**Kind:** Process, Surface.

The citable source for the read path off a live Drone: where the turns are teed from, what is kept, and what a person sees.

The problem it solves: [Fleet](fleet.md) reads a Drone's transcript to detect evidence and advance the Job, then drops it. A person who wants to know what the Drone is doing right now has [Pilot](pilot.md), which ends autonomous execution to answer the question — the heaviest possible instrument for a look.

## Observe is not Pilot

**Observe reads; Pilot takes over.** Why: the two answer different questions, and only one of them costs the Job its Drone.

| | Observe | Pilot |
| --- | --- | --- |
| The Drone | Keeps running, untouched | Terminated, or parked |
| The worktree | Fleet's | The engineer's |
| The Job status | Unchanged | `piloted` |
| Autonomous progress | Continues | Ends |
| Recorded on the Job | Nothing | A transition, with its reason |
| Reversible | Close the view | No |

**A Drone is never told it is being observed.** Why: a read that changed what was read would not be a read, and the prompt contract has no clause for an audience.

## One read, three consumers

**Fleet's line loop stays the only reader of the Drone's stdout.** Why: Fleet parses that stream to detect evidence and advance the Job, and a viewer consuming from it would starve the thing that makes progress.

The tee sits where Fleet already holds the line. `Started.transcript` in `crates/fleet/src/drone.rs` is read a line at a time and handed to `AgentHarness::read`, which takes the line as `&str` and answers with `Vec<DroneEvent>`. Both values are in hand at that call site, and each consumer takes the one it needs.

| Consumer | Takes | For |
| --- | --- | --- |
| The parser Fleet already has | The parsed events | Evidence, refusals, completion, liveness |
| The live view | The parsed events | Rows, while somebody is watching |
| The Drone's transcript | The parsed events | The durable record |

**Every consumer takes the parsed events, and the raw line reaches none of them.** Decoding is itself the redaction: spike 3's opening event carried the operator's home path and whole tool inventory as raw text, and `DroneEvent::Started` carries a session, a model and a count. A file of raw turns would need scrubbing before anything could read it, which is the cost `../spikes/003-does-headless-output-parse.md` paid by hand.

What that gives up is a payload the vocabulary has no variant for: `Unrecognised` keeps the kind and drops the body. A stream that grows an event is visible as a count, and re-reading it needs the stream again rather than the file.

**Nothing decodes the stream twice.** Why: a second decoder is a second vocabulary, and the two disagree on the line that matters.

**Adding a consumer never changes what Fleet does with the events.** A tee that could drop a line the parser needed would make watching a Job change its outcome.

## Where a transcript lives

**`<repo>/.armada/transcripts/<drone-id>.jsonl` — a separate artifact, not a wrapped log line.** The Job log at `<repo>/.armada/logs/<job-id>.jsonl` keeps its envelope unchanged. Why: a transcript is far larger than everything else a Job emits and carries its own retention question, so wrapping it would leave the Job log neither readable nor greppable.

**A retry is a second `drone_id` under one `job_id`**, which [Log Envelope](log-envelope.md) already states — so a retry gets its own file rather than interleaving with the first, and nothing ever appends to a transcript whose Drone is gone.

| | |
| --- | --- |
| **When it is written** | While the Drone runs, from Fleet's line loop. Not at the end, and not on request |
| **What it contains** | One row per `DroneEvent`, in the order the Drone emitted them, each with the instant Fleet saw it and the step that was running then. Never the wire shape |
| **Who writes it** | Fleet, and nothing else. It is opened before the Drone is spawned, so a disk that will not hold the record escalates the Job instead of losing a transcript quietly |
| **What reads it** | Observe's backfill, and Debug's turn history. Nothing decodes it — the rows are already the vocabulary |
| **What names it** | A line in the Job's log carrying `job_id`, `drone_id`, `step_id` and the path in `fields`. **The only record of the path**: `assigned_drone` on the step's row names the Drone while it is running, and is null again once it has exited — which is now at every step boundary, so a Job of four steps has four transcripts and that pointer can name at most one of them |

**Fleet going away does not corrupt it.** Each row is flushed as it is written, so what was taken is on disk; the writer goes with Fleet, and Fleet never puts a Drone back onto a Job it did not spawn. A closing line in the Job log says how many rows the file holds.

### The step a row was written under

**Every row carries the step that was running when Fleet saw the line.** The reason has changed and the rule has not.

It used to be forced: one Drone worked a Job's steps in turn, so a step read at the moment a row is read back was the wrong answer for every row but the last, and a step copied in when the Drone was spawned was the wrong answer for every row but the first — which is what shipped, and it made a four-step Job's transcript read as though all of it happened during the first step.

**A Drone belongs to a workflow step** ([Drone](drone.md)), so a transcript file no longer spans steps and copying the step in at spawn would now be right for every row. The label stays anyway, for two reasons that are about reading rather than about correctness: a row is self-describing, so nothing has to join it to the line that named the file to know where it came from; and the alternative is a fact stored once per file that every reader has to carry, which is the shape that produced the defect above.

**It is a label on a row and never a range.** A step can be run more than once, so the same id may appear, stop appearing, and appear again; nothing may read a transcript as one contiguous span per step.

**The record is told the step rather than asked for it when a row is written.** Why: the row is built by the loop pumping the Drone's output, and a lookup back into Fleet on every line is a different design with a different cost. It is told once, when the Drone is put on its step — there is no move to record within one Drone's life, because the Drone ends when the step does.

**A step that advances mid-turn labels what arrives after it.** The row belongs to the moment Fleet saw it, so the turn that was in flight stays with the step it was in flight under and the answer that comes back belongs to the new one.

**Rows written before Fleet recorded a step carry none, and nothing relabels them.** Their true step is not recoverable from the file, and naming the step the Drone was spawned on would state as fact the thing this record exists to correct.

### When the sink cannot keep up

**A row the queue will not take is dropped, counted, and written down as a `missed` row.** Never awaited: the loop that hands rows out is the loop that advances the Job, so a sink that could hold it up would make watching a Job change its outcome.

**A gap is stated rather than left.** The count lands in the transcript among the rows it was lost between, and the total is on the Job log's closing line — because a transcript that quietly skipped rows reads as a Drone that went quiet, which is the one thing this record exists to tell apart.

## What a viewer sees

**The rows are `DroneEvent`, not `stream-json`.** The vocabulary in `crates/adapter-traits/src/event.rs` is already the distinction a reader wants — a call, its answer, prose, a refusal — so Bridge is handed named events and never raw transcript.

**Three rows are not a `DroneEvent` at all, and they are the other two voices.** A step is Armada opening it, the Drone working, and Fleet running the Checks and reading what came out. Only the middle one was written down, so a surface drawing the activity log had the Drone's turns and nothing about what it had been asked or what was made of it. Every row carries a `by` saying which of the three it is; a row written before that field existed is a Drone's, which is what the default says rather than guesses.

| Row | Carries | Reads as |
| --- | --- | --- |
| `Called` | The tool, the call's id, and what the call was on — a path, a command, a pattern — with a flag saying whether it was cut | The Drone reached for something, and what it did with it |
| `Answered` | The call's id, whether the tool itself failed | What came back |
| `Said` | The Drone's own text | Prose, which advances nothing |
| `Refused` | The tool, the call, the harness's wording | Reached for and stopped |
| `Started` | Session, model, MCP server count | The session, and whether confinement held |
| `Unrecognised` | The kind | The stream carried something the vocabulary has no variant for — a new event, or the Drone's reasoning |
| `Unreadable` | The line as the decoder saw it, and why | A line that did not decode |
| `Ended` | The turn count, what the run cost, and how many turns the harness refused | The Drone stopped, and what it spent getting there |
| `Instructed` | Which turn it was, and what Armada told the Drone, whole | Armada spoke into the session |
| `Checked` | One declared Check, as Fleet ran it — the same `CheckRun` a step's panel carries | Fleet ran something the Drone never can |
| `Produced` | What the worktree held, by name and change kind | Fleet read what the step made |

**`Missed` is the one row that is not a `DroneEvent`.** It is the sink saying how much of the record is not there, which the vocabulary has no way to say. It is written to the file and is not shown to a viewer, whose losses are the subscription's rather than the sink's.

**What Armada said is carried whole and is not bounded.** Why: it is Fleet's own rendering of a template Fleet holds, not something a Drone chose the size of — the bound `CallDetail` puts on a call's arguments exists because a `Write` argument is a whole file, and no such hazard is here. Truncating the one thing the step was asked to do would make the row unreadable for exactly the question it answers.

**A Check row is written where the ruling is, and the reading of the worktree beside it.** Why: that is once per submission rather than once per turn, and it is the moment both are true — the Check has an outcome and the worktree holds what the step made. A worktree that will not open writes no row and fails no ruling, which is the rule the live footprint reading already follows.

**A tool call and its result are one row, joined on the call id.** Why: `Called` and `Answered` arrive as separate events with a gap between them that is the tool running, and two rows would separate a command from its output by everything that happened while it ran.

**A call says what it did, and the payload is bounded by its type.** A row carrying only a tool name and an opaque id reads the same whether the Drone ran `ls` or `rm -rf`. What is carried is the argument a person would name the call by — the path, the command, the pattern, an edit's size as `+42 -18`, and, for Armada's own tools, the paths a step declared and what it claimed. What is not carried is content: a `Write`'s file body has no field to arrive in. `adapter_traits::CallDetail` is the bound rather than a check at the writer, so there is no call site at which an unbounded argument can reach a row; a detail longer than the bound is cut, and **the row says it was cut** rather than leaving a reader to infer it from a trailing character a command could legitimately end with.

**Armada's own tools are read from the same list as every other tool's.** Why: they were not, and every `mcp__armada__*` call recorded an empty detail — the blind spot was exactly on Armada's own surface. The decoder reads a fixed set of argument keys, Armada's tools use keys of their own, and a key with no field is a key that is dropped. What `declare_scope` was called with is **recoverable nowhere else**: no route, no log line and no store field holds it, so a drift refusal could not be read against the claim it refused. `run_checks` still records nothing, and that is the truth rather than the gap — it takes no arguments at all.

**A path under a home directory is elided to `~`.** Why: spike 3's finding was the operator's own home path in the opening event, scrubbed by hand before this repository could hold the capture. A row naming a file the Drone read would carry it again on every call.

**`Unrecognised` and `Unreadable` are rows and never gaps.** Why: they are already never dropped in the vocabulary, and a view that hid them would report a quiet stream where there was a broken one.

**The rule holds at the block and not only at the turn.** A turn is several content blocks, and a decoder that dropped the ones it had no variant for and synthesised a row only where the turn had nothing else would lose them on every turn that also made a call — which was the commonest turn in the stream. So each block answers for itself, and the three answers a reader can tell apart are a turn that reasoned, a block the vocabulary does not name, and a turn that held nothing at all.

**A turn that carried reasoning says so, and does not carry it.** Why: reasoning is the model's working rather than its answer, it is long, and `../scope.md` records that reading a transcript is the thing Armada exists to escape. What a reader cannot do without is knowing the turn had reasoning in it — a turn that thought for thirty seconds and then made one call rendered as one call, and nothing said anything had been taken out. Whether the text is ever shown is open; see Open questions.

### Not shown

| Withheld | Why |
| --- | --- |
| `QuotaMoved` | Belongs to dispatch gating, not to this Job |
| Raw `stream-json` | The row vocabulary is the view; nothing needs the wire shape |

**`Ended`'s cost and turn count were withheld here, because the Job's rail was said to state them.** It states a step's state and how long the step took, and no other row and no other shape on the wire carries a spend at all — so the one irreversible thing about a Drone was reachable from nowhere, and a Job that burned a dollar over forty turns read exactly like one that gave up in four. The row is shown.

**A row is one Drone's run and never a Job's total.** A Job has a Drone per step and a retry adds another, so an ordinary four-step Job has four rows before anything goes wrong; what a Job cost is the sum, and where that sum is drawn — a scanned Board, or a finished Job's outcome — is the surface's question rather than this one's.

## Observing is not a Job state

**Nothing about observing is written onto the Job.** The status stays whatever it was, no transition fires, and `crates/core-model/src/job/transition.rs` gains no edge.

Why: `piloted` is a status because takeover changes who is driving and what happens to the worktree, which the record must survive a restart knowing. Observing changes neither, so a status for it would be a state nothing could act on.

**A Job does not know how many people are watching it.** Watching is a property of a connection, not of the work.

## Nobody is watching

**The log is written whether or not anybody is looking.** Why: the durable record is what makes the history readable later, and a record that existed only while a window was open would be missing exactly when someone came to read it.

**The broadcast with no subscriber is dropped and nothing is told.** That is already how publishing behaves in `crates/api/src/stream.rs` — never blocks, never fails, drops with nobody listening — because the channel is a notification of a durable record rather than a copy of one.

## Joining a Job already running

**One connection answers with the history and then the live rows.** The backfill is read from the Job's log; the live rows come from the subscription; a viewer opens one thing and gets both.

**The subscription is opened before the log is read.** Why: the other order loses a row that arrives between the two, and this is the ordering the event stream already documents for the same reason.

**A row may repeat and never goes missing.** A duplicate is detectable from the call id it carries; a gap is not detectable at all.

## Not the global event stream

**Observe does not travel `/events`.** Why: that stream is one drop-oldest channel of fixed capacity carrying every Job, so transcript rows at Drone speed would evict the state changes Bridge draws the Board from.

An eviction there is not a lost row. It is a `Missed` count and a full resync of every Job — the cost the bound exists to make visible, paid continuously for as long as anyone watches anything.

**Observe takes a per-Job channel, subscribed to only while somebody is watching.** The rule it departs from — one global stream, a client subscribes to nothing — was made because the Board renders every Job, so a subscription would put state on a connection whose value is being cheap to drop. Neither half holds here: a viewer is opened on one Job deliberately, and dropping the connection ends the watching rather than corrupting a view.

**Bridge stays a scanning surface.** Observe is not on the Board; it is opened on one Job and closed, the same shape the turn-level detail rule already has.

## Secrets

**Observe adds no sink outside the redaction the architecture already declares.** `Redactor` scrubs the per-Job log and the event stream, which are Observe's two consumers, and `Secret<T>` cannot be serialised into either.

**What redaction bounds is what Armada issued.** A transcript carries whatever the Drone read, so a Drone that reads a credential out of a file puts content into the stream that Armada never brokered and cannot recognise.

Two facts bound it rather than a mechanism. The view is local, on the operator's own machine, reading a Job the operator dispatched. `../spikes/003-does-headless-output-parse.md` measured the concrete instance — the session's opening event carried the operator's own tool inventory and home path — and that is the `Started` row.

**Nothing here makes a transcript shareable.** See Open questions.

**Redaction is not yet a step on this path.** `Redactor` is named by the architecture and does not exist in the code, so nothing scrubs the transcript or the Job log today. What bounds the exposure is the decoding: a row is a `DroneEvent`, and the variant that carried the operator's home path and tool inventory in spike 3's capture carries a session, a model and a count.

**Carrying a call's arguments widens what is inside that bound, and does not move the bound.** The socket already carries `Said` — the Drone's own prose, which can hold anything it read — and `Unreadable`, which is raw. So arguments are not a new category. What is new is the path: a credential in a command line now reaches a row because the Drone *used* it, where before it reached one only if the Drone *talked about* it. The answer is still loopback-only and a viewer who is the operator, and this makes attaching a transcript to anything more costly rather than less — see Open questions.

**Armada's own turns widen nothing.** A brief is rendered from the Job's facts, its acceptance criteria and its frozen workflow, and `get_job` already serves all three in full to the same viewer — `crates/ipc/src/detail.rs` argues that case for `facts` on its own terms. What is new here is that the rendering itself is readable, not that a value reaches a reader who could not already ask for it.

## Neither a Doctor row nor an Alert

**Observe earns no [Doctor](doctor.md) row.** Why: a row exists where Armada depends on a thing and that thing can be up or down, and a view is neither depended on nor a service.

**Observe raises no Alert.** Why: an Alert is a queued decision on a Job, and watching asks for no decision.

**A view that cannot open is an error, not a health state.** It reaches a person through the error contract, at the moment they tried.

## Where it surfaces

**Observe is the continuous view; Debug is the snapshot.** Debug is reached reactively, adapts its layout to the trigger that fired, and answers where the Drone got to. Observe is opened deliberately on a healthy Drone and answers what it is doing now.

**Debug's turn-history layout has no source until Observe keeps one.** Fleet drops the transcript today, so what Observe writes is what Debug reads.

The flow, the placement and the copy belong to the Monitor Active Work user journey. The read-only rule is shared with Debug and stated there.

## Open questions

- **[observe-transcript-sharing]** May an observed transcript leave the machine, and what would have to be true first? Redaction covers credentials Armada brokered, and a transcript carries whatever the Drone read — so the bound today is that the viewer is the operator, on their own machine, reading a Job they dispatched. Attaching a transcript to an escalation, a bug report or a shared Job record would each remove that bound. `../spikes/003-does-headless-output-parse.md` is the worked example: its capture had to be redacted by hand before this repository could hold it, and what needed removing was the session's opening event. Since a `Called` row carries the call's arguments, a shared transcript would also carry every command the Drone ran and every path it touched — which is a wider scrub than the opening event, not the same one.
  **Something now waits on it.** The filing review built for an error offers a
  transcript row and offers it *withheld*, named on screen with this as the
  reason — turning it on by default would answer this question by shipping. Two
  things have to be settled together: whether it may go at all, and **how much
  of it goes**, which is undecided and has no defensible answer today. The only
  bound in the codebase is `HISTORY` in `crates/fleet/src/transcript/backfill.rs`,
  and it sizes a viewer's backfill in rows rather than a filing in turns, so it
  is not the number. See the Error Contract's `[filing-transport]`: until
  something can send, nothing leaves the machine by this route anyway, and the
  row's default is the whole of the exposure.

- **[observe-reasoning-content]** Does a viewer ever get to read a Drone's reasoning, or only that there was some? The row says the turn reasoned and carries no text, which is the smaller of two readings. The larger one — a collapsed row that opens onto the working — is what a person actually wants when a Job went somewhere surprising, and it is also the whole of the transcript-reading this product exists to escape, at a size no other row comes near. It would need a place in the vocabulary that is not `Unrecognised`, a `Saw` variant with `enum-verbs.toml` rows, and an answer to `[observe-transcript-sharing]` that today's rows do not need, because the working is the one field that has never been bounded by anything.

- **[observe-retention-against-the-log]** Does a Job's transcript live and die with its log, or on its own schedule? Per-Job logs are pruned on terminal status after a retention grace period, which would take the transcript with them — and the transcript is the larger artifact by a wide margin, so the setting that sizes one may be wrong for the other. `log_retention_days` does not exist yet, which is why this is answerable now rather than later.
