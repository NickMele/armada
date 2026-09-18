# Protocol: the Fleet/Bridge seam

Armada has exactly one place where Rust stops and TypeScript starts: the wire
between Fleet (the daemon, Rust) and Bridge (the desktop app, Electron). Every
other boundary in the system is a function call or a file. This one is a
process boundary between two binaries with independent release cadence and
independent lifetimes, and everything in this document exists because that
combination has already gone wrong once, in v1, and cost real debugging time
figuring out which side was lying.

If your change touches `protocol-version.toml`, the (forthcoming) `ipc` crate,
anything under `apps/desktop/src/preload`, or the WebSocket event stream, read
this first.

## The single source of truth

`protocol-version.toml`, at the repo root, holds a major and a minor:

```toml
major = 4
minor = 0
```

**Which of the two moves decides what a mismatch does**, and the table further
down is what the code implements. `major` moves when a message an older peer
already parses stops parsing the same way. `minor` moves when the change is
additive only, and resets to zero whenever `major` moves.

The pair crosses the wire as **one field carrying both numbers** —
`"protocol_version": {"major": 4, "minor": 0}` — rather than as two fields. Two
would let either side compare the majors and forget the minors, which is the
defect this shape replaced: the version was one integer, `connection.ts`
compared it with `!==`, and every bump was a full refusal. A bare integer is
still read, as that major at minor zero, because version 4 shipped as one and a
Fleet from before the pair should reach the skew screen rather than read as a
runtime file nothing wrote.

That file is read on both sides, but not the same way:

- **Rust** reads it at compile time. `crates/ipc/build.rs` parses it and emits
  the two numbers, which `crates/ipc/src/version.rs` assembles into the
  `PROTOCOL_VERSION` constant the rest of the Rust workspace compiles against.
  This half is self-correcting by construction — `build.rs` runs on every
  `cargo build`, so the embedded constant cannot go stale relative to the file.
  There is no step to forget here.
- **TypeScript** cannot read a `build.rs`. The plan is a codegen step, driven
  off the same `ipc` crate that defines the DTOs, that emits the matching
  TypeScript types and the version number into `packages/` (see
  `packages/README.md`: "the generated IPC types" is named as the reason that
  directory exists). **Both generated outputs — the Rust constant's TS mirror
  and the DTO types — are checked into the repo, not generated at build time
  on the TS side.** A generated file that's `.gitignore`d looks fine locally
  and is wrong on every machine that didn't just run codegen.

Because the TypeScript half is generated-then-committed, it can drift from its
source the same way any generated-then-committed file can: someone edits the
`ipc` crate and doesn't rerun codegen, or edits the generated `.ts` file by
hand because it was faster. That drift has shipped once. The major moved to 6,
the constant stayed at 5.7, and a Fleet and a Bridge built from the same commit
refused each other into the lifeboat with every check green.

**`cargo xtask verify-foundations` holds the two numbers together now.** It
reads `protocol-version.toml` and the generated constant, refuses a pair that
disagrees, and names both files, both versions and the command that writes the
file. `xtask/src/rules_protocol/version.rs` is the rule.

**It refuses; it does not rewrite.** A gate that ran the codegen itself would
leave nobody knowing the step exists, which is the same defect one release
later — and the next registry to grow a generated half would ship it again.

Two things about the generated half are still checked by nothing, and
`[verify-protocol-task]` below is where they are named:

1. The checked-in generated TypeScript matches what codegen would produce from
   the current `ipc` source, right now.
2. Nothing outside the generated file hard-codes the protocol version as a
   literal.

That second check would answer a violation this document was written
against: `apps/desktop/src/preload/index.ts` returned a hand-typed `1`, with no
mechanism forcing it to move when the source file did. It reads the generated
constant now, and nothing in Bridge restates either number — but the check is
what keeps it that way, because the literal is a one-line shortcut that looks
harmless in review.

**Contributor workflow, in order:**

1. Change the DTOs in `crates/ipc` (add a field, add a variant, whatever the
   change is).
2. Decide which number moves, from the table below, and move it in
   `protocol-version.toml`. Additive-only moves `minor`; anything else moves
   `major` and resets `minor` to zero. **The table is the decision, not a
   guideline** — a minor bump that removes or retypes a field makes Bridge's
   banner a lie and breaks it while a Job runs.
3. Regenerate the TypeScript with `pnpm --filter @armada/desktop codegen`. It
   needs `pnpm install` to have run and nothing else; it rewrites
   `packages/protocol/src/generated/` and `packages/components/src/generated/`
   from `protocol-version.toml` and
   `crates/core-model/domain/`, and prints one line per generated file plus any
   registry row it could not render. **It emits the version mirror and the
   enum vocabulary, not the DTO types** — nothing generates those from the
   `ipc` source yet, so a shape change is still hand-mirrored on the TS side
   and that is the gap `[protocol-codegen]` names.
4. **Run `cargo xtask verify-foundations`.** It refuses a generated constant
   that disagrees with `protocol-version.toml` and names both versions, which
   is the whole of step 3 for the version. It needs nothing built, and it
   reports on rules that have nothing to do with the protocol, so read the line
   naming the version rather than the exit code. That nothing outside the
   generated file hard-codes the version is still verified by reading —
   `[verify-protocol-task]` below.
5. Commit the `ipc` source change and the regenerated files in the same
   change. A generated-file diff with no corresponding source diff, or vice
   versa, is the thing review should bounce.

## DTOs, not domain types

`WireError` is a DTO like any other, and `docs/contracts/error-contract.md` is
what specifies it —
which fields are guaranteed, why `level` and `component` are not among them,
and why removing an error code is a minor bump. The v0 lifeboat below is
deliberately outside that contract.

`ipc` speaks its own vocabulary. It does not re-export `core_model::Job` and
put it on the wire. The conversion is explicit, one direction, and lives at
the Fleet boundary:

```rust
// crates/ipc — the DTO. Only what Bridge is allowed to see.
pub struct JobSummary {
    pub id: JobId,
    pub status: JobStatus,
    pub drone_label: String,
    pub started_at: DateTime<Utc>,
    // no working directory, no adapter credentials, no raw transcript path
}

// Fleet — the conversion, and the only place it happens.
impl From<&core_model::Job> for ipc::JobSummary {
    fn from(job: &core_model::Job) -> Self {
        ipc::JobSummary {
            id: job.id,
            status: job.status,
            drone_label: job.label.clone(),
            started_at: job.started_at,
        }
    }
}
```

The reason this conversion has to exist, rather than serializing
`core_model::Job` directly, isn't code cleanliness — it's that `From` is where
someone has to decide, field by field, what a Bridge (running on someone's
laptop, potentially screen-shared, potentially logged) is allowed to see.
`core_model::Job` will accrete fields as Fleet's needs grow: filesystem paths,
adapter tokens, internal retry state. If that type is `#[derive(Serialize)]`
and put straight on the wire, every new field is redacted or not by accident —
whichever `serde` does by default. `From<core_model::Job> for
ipc::JobSummary` forces a human to look at the new field and write a line of
code, one way or the other. A domain type on the wire is a redaction decision
nobody made.

This cuts the other way too: `ipc` types have no business back in
`core-model`. If Fleet-side code needs a `JobSummary` to build a response,
that's an argument for a thin builder function, not for teaching `core-model`
about the wire's shape.

## Minor vs. major

A minor bump means: **every message an older peer already knows how to parse
still parses the same way.** That is the entire mechanism behind Bridge
running against a newer Fleet with nothing worse than a banner — Bridge parses
fields it recognizes and ignores fields it doesn't, so an additive change is
invisible to it. The moment a bump changes the meaning or presence of a field
an old client already reads, "ignore what you don't recognize" stops being a
safe strategy, and that's a major bump — the lifeboat, not a banner.

| Change | Minor or major | Why |
|---|---|---|
| Add a new DTO / new route / new event type | Minor | Old peer never looks for it, never sees it |
| Add an optional field to an existing DTO | Minor | Old peer ignores unknown fields; new peer treats absence as valid |
| Add a new enum variant, where the enum is only ever *written* by this side and *read as opaque* by the other | Minor, with a caveat — see below | Depends entirely on how the other side matches |
| Add a new enum variant the other side is expected to `match` on | **Major** | An exhaustive `match` on the old side has no arm for it — compile error in Rust, silent `undefined` branch in TS |
| Make a required field optional | **Major** | Anything already relying on its presence (including old Bridge's own type assumptions) now sees a value that used to be guaranteed |
| Make an optional field required | **Major** | Old messages that omitted it become invalid under the new contract |
| Rename a field or a variant | **Major** | Identical to removing the old name and adding a new one — the old name silently stops arriving |
| Change a field's type (including widening, e.g. `u32` → `u64`) | **Major** | "Widening" is a Rust-only intuition; on the wire it's a different JSON shape and a different TS type, and the old side's deserializer doesn't know it's "compatible" |
| Remove anything | **Major** | The obvious case, included for completeness |

**An optional field is left out when it is empty, never sent as `null`.**
Bridge types it `field?: T` and compares against `undefined`, which `null`
passes. Every `Option` on a DTO Fleet writes carries `skip_serializing_if`, or
`deserialize_with = "stated"` where `null` is a value a person chose.
`xtask/src/rules_protocol/nulls.rs` is the rule.

The three people get wrong most often: widening an enum "because it's just
adding cases," making a field `Option<T>` "to be safe," and renaming a variant
"for clarity." All three feel non-breaking from inside the change and are not.
If you catch yourself writing "this shouldn't break anything, it's just
adding/loosening X" — that sentence is the tell. Stop and check whether the
other side's code has an exhaustive match, a presence assumption, or a name
lookup anywhere near the thing you're touching.

**The caveat row has exactly two instances, and both are deliberate.**
`FleetCapacity.held_by` — which one of the concurrency bound, memory or disk
is stopping the next Drone — is a `String` on the wire rather than a
`wire_enum!`, and `crates/ipc/src/capacity.rs` is where that is argued. Fleet is
the only writer, Bridge looks the value up in the generated vocabulary rather
than matching on it, and that map already answers `undefined` for a key it does
not hold. So a fifth reason is a `core-model` variant, a row in
`enum-verbs.toml` and a codegen run, and it moves neither number here.

**`JobSummary.queued_reason` is the second**, since `frozen` joined it. Bridge
types it `string` and reads it through the same generated vocabulary, and the
only Rust readers of a `JobSummary` are this repository's own tests, built at
the same version — so a new reason is minor while nothing branches on it.

**The condition is what makes it minor, not the type.** The moment something on
either side branches on this value rather than rendering it, the row above it
applies instead and widening the set is a major bump. Every other closed set on
this seam is the strict kind and refuses a spelling the registry does not have,
which is right for `JobStatus` — Bridge picks a screen from it.

**And the test is where the set's growth comes from, not how it is read.**
`JobSummary.resumption` — which act a person took to put a `queued` Job back —
is rendered exactly as opaquely as `held_by` and is still a strict
`wire_enum!`, because its three values are the shapes the inner step machine
can be in. A fourth would mean that machine grew a state, which is a change
every reader has to be told about. `held_by`'s set grows every time Fleet
learns to read another resource, which is a change no reader needs to be told
about at all. Ask which of those two a new set is before making it open.

## What Bridge does with the version it reads

Bridge is the side that decides. It reads Fleet's version out of the runtime
file **before it opens a socket**, so a refusal is a screen naming both versions
rather than a malformed first message, and it checks the same fact again on the
resync — a client that reached the socket without reading the file has had no
check at all, and a Fleet restarted under a live socket is not the Fleet the
file described.

Four readings, and only the first two connect.

| Reading | What is true | What Bridge does |
|---|---|---|
| Same | The majors and the minors agree | Connects. The Fleet panel says nothing about versions |
| Fleet ahead | Same major, Fleet's minor is higher | **Connects, and carries a banner.** Everything drawn is current; Fleet has additions this Bridge cannot ask for |
| Fleet behind | Same major, Fleet's minor is lower | **Refuses.** The screen names both versions and says to restart Fleet when no Job is running |
| Incompatible | The majors differ, either way round | **Refuses.** The screen names both versions and says to update both to the same commit. This is what the v0 lifeboat is for |

**The middle two rows are the same gap in opposite directions and they are not
the same situation.** Additive-only says the newer side's additions are things
the older side never asks for and never reads. A newer *writer* is therefore
safe: Fleet sends a field, Bridge ignores it, and nothing Bridge draws is
wrong. A newer *reader* is not: Bridge reads a field an older Fleet was built
before sending, and additive-only promises nothing about that. The hole would
arrive mid-Job rather than at startup, on a Job Board that gives no sign it is
missing anything — which is worse than not connecting.

The banner therefore says the connection is fine and names what it cannot
reach. It goes in the Fleet panel beside the running dot, as advice on a
healthy connection, and **not** as a failure notice: a minor gap Bridge can
survive is not a fault, and drawing it as one tells somebody something is broken when it is
working. `packages/shell/src/fleet.ts` carries the sentences and
`packages/protocol/src/version.ts` carries the rule; `crates/ipc/src/version.rs`
is the same rule in Rust, where the four readings are tested.

**The rule is spelled twice, and that is a known cost.** Bridge decides, so the
rule has to exist in TypeScript; the desktop app has no test runner, so the only
place the four readings can be proved is Rust. Two spellings of one rule is
exactly what this repository calls a second vocabulary, and it is written down
here rather than left to be discovered.

## Why skew is dangerous here specifically

Version skew is usually a deploy-time annoyance: you restart the old thing,
it's fine. That's not what happens here, because **Fleet outlives Bridge by
design.** Fleet is a daemon; Bridge is a window someone closes to go to lunch.
A Job runs unattended, with Drones spending real tokens against real API
budgets, for however long it takes — hours, sometimes. Fleet gets upgraded
during that window because that's when upgrades happen: nobody's watching.

So the skew window isn't "between deploys," it's "for the entire duration of
whatever Job happens to be running when someone updates Fleet." A major-bump
skew discovered mid-Job doesn't get a graceful restart — the connection that
was streaming Drone events goes bad while a Drone is mid-tool-call, burning
budget, with nobody able to see what it's doing until Bridge reconnects
through the lifeboat and can offer nothing better than "kill it." That's the
cost minor-bump-additive-only is bought against: a minor bump has to be safe
to hit *mid-Job*, unattended, with money on the line, not just safe to hit at
startup.

**And the same lifetimes make the refusing direction the likely one.** A
running Fleet's version does not change when someone updates the app — the
daemon that was started last week is still speaking last week's protocol, and
the Bridge relaunched after the update is the newer of the two. So "Fleet
behind" is what an ordinary update produces and "Fleet ahead" is the rarer
case, reached by restarting Fleet without relaunching Bridge. The banner is not
the common path. The refusal is, and its screen has to say plainly that the
daemon is the thing to restart.

## The v0 lifeboat

When the version check refuses — either of the bottom two rows above — Bridge
doesn't get nothing. It gets four routes that don't depend on version
agreement:

| Operation | Route |
|---|---|
| List Jobs with status | `GET /v0/jobs` |
| Kill a Job | `POST /v0/jobs/:id/kill` |
| Stop Fleet | `POST /v0/stop` |
| Report Fleet's version | `GET /v0/version` |

That's the whole surface. Bridge's recovery screen is built on exactly these
four: show what's running, name both versions so the human can tell what's
mismatched, and offer per-Job kill so nothing is left burning tokens
unsupervised while someone goes and fixes the mismatch.

The lifeboat's entire value proposition is being the one thing guaranteed to
work when everything else — the ipc types, the codegen, the version
negotiation — has already failed or gone stale. That guarantee has exactly one
precondition: **the lifeboat itself never needs to change.** Concretely, that
means:

- **Hand-written, not derived.** No `ipc` types, no shared serialization
  helper, no codegen. If the machinery that generates the rest of the
  protocol breaks, that must not be able to take the lifeboat down with it.
- **`curl`-testable.** Plain JSON over plain HTTP, no auth handshake beyond
  whatever's already required to reach Fleet at all, no client library
  required to exercise it.
- **No events, no streaming.** A WebSocket is exactly the kind of stateful,
  versioned, buffered thing the lifeboat exists to not depend on.
- **No new dependency, ever.** A second reason gRPC was rejected for the main
  protocol was that it would have put a codegen toolchain underneath the one
  route table that's supposed to have none. Don't reintroduce that by way of
  the lifeboat.

What would break the guarantee: adding a fifth operation because it seemed
convenient, pointing any of the four routes at `ipc` types "just to reuse the
struct," giving the lifeboat its own version number that then itself needs
negotiating, or letting it grow an auth or session concept that the main
protocol also has to keep in sync with. Every one of those is a small,
reasonable-sounding change that turns four static routes into a second thing
that can be stale. If a change to this document's four rows is ever proposed,
that's the signal to slow down, not speed up.

## The first socket: every Job's state

`GET /events` is the stream Bridge draws the Board from. Its shape is
`crates/ipc/src/event.rs`, which points here.

**A reconnection resyncs; it does not replay.** Every connection opens with a
`Resync` carrying the current state of every Job and the cursor that state is
current as of. Replay from the beginning was rejected on what it costs mid-Job:
a Bridge reopened after lunch would fold hours of transitions before it could
draw anything, into a Board Fleet could state in one message.

| A resync rebuilds | A resync cannot |
|---|---|
| Where every Job is now, and why | The path taken — no transition history |
| Which step it is on | An instant for anything that already happened |
| Whether a Drone is on it | Anything about a Job retained out after a terminal status |

A surface drawing a timeline reads `get_job_events`, and its timeline begins at
the connection.

**What a resync cannot rebuild, Bridge reads back — every region together.** A
resync is a Board, so nothing under the open Job comes with it, and the
receiving side is what makes the open Job's screen whole again. That reading is
one list in `apps/desktop/src/main/screen.ts`, and it is one list on
purpose: the reads it names were once classified at the call site, one region at
a time, and the region left out was the panel reporting the outage. A read added
to that screen is added to the list.

**A resync arrives on two occasions, and they are not the same recovery.** The
first message on a connection is Fleet coming back, and every read attempted
while the socket was down failed — so a surface can be holding a failure nothing
else will clear. A resync following a `Missed` is a gap under a connection that
held, where HTTP answered throughout, so only what events keep current can be
stale. Bridge tells them apart by which resync it is on the socket.

| Taken again on a gap | Taken again only on a reconnection |
|---|---|
| Everything an event re-reads: the Job whole, what it holds, its history | The reads only a press makes, and only where one is showing a failure |
| Either per-Job socket that is down | |

**A reconnection is not a refresh.** Re-reading every open surface on every
resync spends the bytes the `get_job`/`get_diff` split exists to save, and a
flapping Fleet turns that into a fetch every retry. What comes back is what a
screen is showing and cannot repair itself.

**The stream is global, and a client subscribes to nothing.** Bridge holds
exactly one connection and the Board renders every Job on it. A per-Job
subscription would put state on a connection whose whole value is being cheap
to drop and remake, and would need a subscribe message, an unsubscribe message,
and a rule for what a resync means when the set changes mid-stream. The one
place a per-Job subscription *is* right is the second socket, below.

### A kind exists when something produces it

**An event kind that never fires reads as a stream that is working**, so a kind
`crates/ipc/operations.toml` names is not stubbed until a record exists for it
to carry. The kinds still waiting describe records this workspace has no type
for.

Two of the produced kinds are there because their absence was a specific
defect, and both are the same defect: what changed most during a run was what
the stream did not carry. A Job running four steps emitted one event until
`job.step_advanced` arrived; a Board could show which Drone was on a Job only
by re-reading the Job until the Drone lifecycle pair did.

`job.files_changed` is the only kind describing a worktree rather than a
record. **Bridge does not read a worktree** — no surface on the far side of
this seam opens a repository, so a file list reaches one only as an event.

### `job.created` is a kind, not a state change

A Job proposed while a client was connected never reached it: creation
published nothing, so nothing woke Bridge and the row appeared only when
something else forced a re-read.

Publishing a `JobStateChanged` instead was rejected on what that message would
have to say. The type has a `from` and a `to`, and a created Job has no `from`
— the honest fields would be `from: awaiting_approval, to: awaiting_approval`,
a transition the edge table does not contain, from a status the Job was never
in. Every client folding the stream would apply a move that did not happen. **A
creation is a row appearing, not a row moving.**

It carries the whole `JobSummary`, because a kind naming only an id would make
every client fetch the row it was just told about.

`job.forgotten` is the opposite message and carries the opposite payload: only
the id. A forget is a real deletion through `Store::forget_job`, and by the
time the event is published there is no row left to carry. A client drops it
rather than replacing it.

### A fact about the fleet is a reading, not only an event

`manifest.reread` says what Fleet's last read of `armada.yml` came to. It is the
second kind naming no Job — `proposal.moved` is the first — and unlike that one
it names no Drone and no step either: a Manifest is Fleet's own, so nothing on
the Board moves when it arrives.

**It is served as well as published, and that pairing is the point.** A refused
Manifest is not an instant that passes. The file on disk and the values Fleet is
running with go on disagreeing until somebody corrects the file, so the fact
outlives any window that happened to be open when the save landed. An event
alone would reach only whoever was looking; `get_manifest_reading` is what a
Bridge opened a minute later asks. `FleetCapacity` is the same shape one fact
over, and a doctor result would be the third.

The rule that falls out: **a fleet-wide fact that persists gets a route and an
event, not an event alone.** A fact that is true only in the instant it happens
— a call going out, a step advancing — needs no route, because there is nothing
left to ask about.

## The unmeasured risk: the WebSocket sink has no back-pressure

This hasn't bitten anyone yet, which is exactly why it's the most dangerous
item here — nobody has a number for it. `axum`'s WebSocket sink is unbounded
on the application side: if Fleet pushes events faster than Bridge's socket
drains them, the server-side buffer just grows. Nothing in the stack currently
pushes back.

Picture several Drones running against a Bridge that's been minimized to the
tray, or is on a slow connection, or is just a slow renderer under load.
Fleet keeps producing tool-call and status events at Drone speed. Bridge
drains them at UI-thread speed. There's no mechanism that notices the gap and
does anything about it — the buffer absorbs the difference until it doesn't.

The fix is built and the risk is still unmeasured, which is the state to read
this in. `crates/api/src/stream.rs` publishes through a **bounded broadcast
with drop-oldest**, so a slow consumer loses old events instead of growing
Fleet's memory without limit, and a drop comes with the message the paragraph
below asks for: *you missed N events, resync*. What nobody has is a number —
how many events a real fleet produces against a real minimised Bridge, and
whether `BACKLOG` absorbs it. The reason this matters as a protocol concern
and not just a
performance one is what happens without it — a reconnecting Bridge that
silently believes its event log is complete will render a Job Board that's
quietly wrong, and "quietly wrong" is worse here than "visibly stale," because
nothing on screen tells the person to distrust it.

If your change touches the event stream, say explicitly whether it makes this
better (bounds something, adds a resync signal) or worse (adds another
unbounded queue, another place assuming delivery is complete).

**`get_events_since` made it better, and is the shape to copy.** An agent
cannot hold a socket, so it polls, and what it polls is a second bounded
window beside the channel: positions and kind names rather than events,
`TALLIED` of them, and a caller whose cursor fell off the back is told how many
it cannot be told about. Nothing is queued per client and nothing is retained
for one that never comes back.

## The second socket: one Job's turns

`GET /jobs/:job_id/observe` is a WebSocket upgrade, and it is the one query in
`operations.toml` whose transport is the socket. It answers with the turns a
Job's Drones have already taken and then continues with the ones that follow,
so joining a Job already running takes one connection rather than a history
call and a subscription that have to be stitched together.

**Who reads it.** A person, through Bridge, on the machine Fleet is running on.
`agent_access` is `No`: a Drone's whole transcript streamed into a session
stays in that session for the rest of it, and `get_drone` serves the snapshot
instead.

**What it needs.** A running Fleet and a Job id. Nothing else — a Job with no
transcript is served, and so is one whose Drone is gone.

**What a viewer sees.** The first message is always `opened`, carrying the
protocol version, the Job, whether a Drone is writing right now (`live`) and
how many older rows the history left out (`skipped`, because the backfill is
bounded). Then the history, oldest first, across every Drone the Job has had —
a retry is a second `drone_id` under one `job_id` and both are the Job's
history. Then the live rows. The connection ends with a `closed` message
saying why, because a socket that simply stops is indistinguishable from one
that broke.

| What happened | What the viewer is told |
| --- | --- |
| A Drone is working | `opened` with `live: true`, the history, then rows, then `closed` / `drone_ended` when the Drone finishes |
| The Job was never dispatched | `opened` with `live: false`, nothing, `closed` / `nothing_writing` |
| Fleet restarted under a Drone that outlived it | The same. Fleet's writer does not reattach and `reconcile` escalates the Job as `interrupted`, so the history is whole and nothing is live |
| The Job id names nothing | **404 before the upgrade**, through the error contract, at the moment they asked |

**Back-pressure, and what is dropped.** Three bounds sit in a row and each is
stated rather than silent. The transcript's file queue drops a row it cannot
take and writes a `missed` row into the file among the rows it was lost
between. The per-Job broadcast channel is drop-oldest, and a viewer that has
fallen behind gets a `missed` message with the count. Neither can slow Fleet's
line loop: the file queue is `try_send` and the channel's send is synchronous
and never blocks, so **watching a Job cannot change its outcome**. What a slow
viewer slows is its own socket task.

**It is deliberately not `/events`.** That stream is one drop-oldest channel of
fixed capacity carrying every Job, so transcript rows at Drone speed would
evict the state changes Bridge draws the Board from — and an eviction there is
not a lost row but a `Missed` and a full resync of every Job, paid
continuously. This is the one place a per-Job subscription is right, and
`docs/concepts/observe.md` is why.

## The run socket: one person's run

`GET /jobs/:job_id/runs/:run_id/observe` is the second socket's shape one
subject over: a run a person started from the run sheet, and
`crates/api/src/watching_run.rs`. It answers with what the run's log already
holds, then the lines the run prints next, then `closed`.

**Who reads it.** The run sheet showing that run, and nothing else.
`agent_access` is `No`.

**Per run, not per Job's runs.** The sheet holds the run's id from
`start_run`'s answer, a run has an end to close on, and a Job has one run out
at a time. A per-Job channel would need the second socket's hand-over between
runs, for a reader nobody has.

| What happened | What the viewer is told |
| --- | --- |
| The run is going | `opened` with `live: true`, the log so far, then lines, then `closed` / `finished` |
| The run has ended | `opened` with `live: false`, the log, `closed` / `finished` |
| The log will not read | `opened`, then `closed` / `unreadable` |
| The id names no run of this Job | **422 before the upgrade**, through the error contract |

**Back-pressure.** The channel is per run and drop-oldest, and a viewer that
falls behind is sent `missed` with the count. The run's log keeps every line
and `get_run_output` reads it, so a drop costs a live line and never the
record. Each live line carries the byte offset just past it, which is what
keeps a line the opening read sent from being sent again.

**It is deliberately not `/events`.** Output is the fastest thing a run makes,
and on the global stream it would evict the state the Board is drawn from.
`/events` carries the run's end, `run.finished`, because that is a fact about a
Job a surface may care about; it never carries a line.

## The server socket: one server's output

`GET /servers/:server_id/observe` is the run socket's shape per server
instance, and `crates/api/src/watching_run.rs` relays both. It opens with what
the server's log holds, then the lines it prints next, then `closed` once the
server has ended. **A server that has ended opens too**, with its whole log,
which is how one that fell over is read.

`/events` carries a server's three lifecycle facts — `server.starting`,
`server.serving`, `server.exited` — each with the whole instance, and never a
line of its output, for the run socket's reason. A Job's servers publish
`server.exited` before the Job's own terminal `job.state_changed`, because they
are torn down before its span is released.

## The Helm socket: one repository's conversation

`GET /helm/observe?manifest_id=` is the second socket's shape with a
conversation for its subject, and `crates/api/src/conversing.rs`. It opens with
the thread so far, then carries every message after it. `POST /helm/ask` takes a
message and answers 202 at once, because the reply is the socket's;
`POST /helm/start_fresh` forgets the stored session and the thread.

**Who reads it.** Bridge's Helm thread. `agent_access` is `No` on all three: a
session must not read or drive another session.

**No end to close on.** A Drone ends and a run finishes; a conversation outlives
every process that answers in it. So the socket carries reply after reply for as
long as a viewer holds it, and `closed` is sent only when a person starts fresh.

| What happened | What the viewer is told |
| --- | --- |
| A person asked | `asked`, then the session's own `row`s, ending in `ended` with what the reply cost |
| The stored session was gone | `fresh` / `session_not_found`, then the rows of the new session |
| No reply came | `unanswered`, with why |
| A person started fresh | `closed` / `started_fresh` |
| The Manifest is not served | **422 before the upgrade** |

**A row is a `Shown` row**, so a reply is read with the vocabulary a Drone's turn
already has. **Back-pressure** is the second socket's: a channel per
conversation, drop-oldest, `missed` with the count, and the thread's file keeps
what a slow viewer lost.

**It is deliberately not `/events`**, for the second socket's reason.

## Protocol 10.11: the review Fleet composed

Job detail's review area and the pull request's description said nearly the
same thing from two independent builders — `#665`. `ipc::JobReview`, additive
on `get_job`'s `JobDetail`, is the fix: Fleet composes one review from the one
reading of the record, in four parts — `why` (the brief, in the requester's
own words), `outcome` (what the worktree changed, as far as a diff can say
it), `risks` (what nothing checked, and what the base carries that this Job
did not write), `evidence` (every step and every Check that ran, with its
outcome) — and renders it two ways: the pull request's Markdown body,
unchanged in content, and this DTO. `crates/fleet/src/review.rs` is the one
builder both readings come from.

Composed at the delivering step's entry, as before, and now also at any other
`human_always` gate, so a workflow that never opens a pull request still
reaches a person with the same four sections. **Absent is a Job that has not
reached a gate yet**, not an empty review — a Job still running, or one that
finished with no `human_always` step at all, carries nothing here, and so does
every Job read from a Fleet older than 10.11.

`VerdictSheet` draws `why` and `risks` into the existing "What you asked for"
and "What proves it" blocks. The Drone's own claims stay their own block,
labelled "What the Drone says it did" and "What the Drone says it left
alone" — the pull request leaves them out on purpose, and the label is what
keeps a Drone's self-report from reading as Fleet's own account.

## Protocol 10.12: keeping a pull request current, and resolving its conflicts

`#663`. Fleet stopped closing and reopening a Job's pull request when main
moved under it — closing and reopening re-pinned the forge's comparison but
never touched the branch, so a pull request behind its base with conflicts
stayed that way. Fleet now rebases the branch and pushes it: clean, in place;
conflicted, the branch is left exactly as it was and a person is told.

`ipc::Currency`, additive on `PullRequestDetail`, names the base a branch was
last brought up to (`rebased_onto`, `rebased_at`) and, where the attempt
conflicted, the files (`conflict_files`) — absent is a branch that has never
needed to move. `resolve_pull_request_conflict` is a new route and `Commands`
method at the review gate: a person sends the branch back for a Drone that
can edit files to bring it current, on the step before the one that
delivers, never the gate's own — #660 found a picked pull-request comment
landing on a summarising step's Drone, which has no git and no code to edit.

## Protocol 11.2: a delivery that skipped its push says so

`#691`. A delivering step's catch-up can conflict with its base after the
step has already made its commit, and `fleet::delivery::deliver` was right to
leave the branch unpushed there — a pull request opened over a conflict is a
review request nobody can act on. What it did not do was say so: the commit
still landed, the Job's log carried nothing about the push it skipped, and a
redelivery onto a pull request that already existed left that pull request
showing the commit before it, silently, all the way to a person approving
work it did not contain.

`ipc::JobDelivery.unpushed`, additive, names why the last commit here never
reached its remote — absent is a push that went out, a repository with no
remote, or a Job that has not reached a delivering step; present is the one
fact the field exists for. Fleet's own record no longer clears `pushed` and
`pull_request` on a skipped push either: nothing about the remote changed
that turn, so the store keeps naming the pull request `resolve_pull_request_conflict`
already knows how to send a Drone back to.

A delivering step whose own catch-up conflicted can no longer carry a Job to
`completed_success` silently: `Ruling::Finished` on such a step is held for a
person instead, and an approval or an override reaching the Job's own ending
while the last commit is unpushed is refused rather than completing over a
pull request that does not carry it.
## Protocol 11.3: a Judge that is unsure asks, rather than stopping the step

`docs/concepts/judge.md`'s asking design, closing #694. A refusal on a
criterion marked `refuse` still stops the step exactly as every refusal did
before this existed; the rest hold the step open at a question instead, and
`declared_plan_drift` can never be marked `refuse` at all.

`ipc::JudgeQuestion`, additive on `get_job`'s `JobDetail` beside
`command_waiting`: the refused criterion, the plain question it asked, the
Judge's own `expected` / `produced` / `consequence`, and when it was raised.
Absent is the ordinary case, and every Job read from a Fleet older than 11.3.

`answer_judge` is a new route and `Commands` method, taking `ipc::JudgeAnswered`
— `answer` (`agree`, `disagree_once` or `disagree_always`) and an optional
`note` that rides along for the record. `agree` fails the step exactly as it
would have without this design; either disagree advances it, and
`disagree_always` also stands the criterion down for the repository, so no
later Job is asked about it either. Refused with a 409 where the Job is not
holding a question open.

## Protocol 11.6: a recording is watched a span at a time

`#615`. A captured video over 20 MiB never played on the Job screen and a
smaller one downloaded whole before a frame of it drew, because every hop read
the file whole: `get_frame` answered the bytes in one response, Bridge's main
process took them into an array, and the renderer made a `blob:` of it. A
two-minute walk through an app runs past that in a minute, so the evidence most
worth watching was the evidence the screen skipped.

`get_frame` now honours `Range` — **on a frame whose media type is a video, and
on nothing else**. A satisfiable span answers 206 with `Content-Range`; one
beginning past the end answers 416 naming the length, which is the one fact a
player can ask again with. A range on any other kind is ignored and the file is
answered whole, which is always legal and keeps the executable kinds — SVG,
HTML — on the single path that has always answered them as bytes.

**Additive, and that is why the minor moved.** An older Bridge sends no `Range`
and gets exactly the response it got before, headers included. The route, the
id and the refusals are unchanged: `kept` still names a row, the record is
still the allowlist that resolves it before a file is opened, and a name no row
of the Job holds still reaches nothing whatever it spells.

**A long span is windowed rather than honoured**, because a player opens a
recording by asking for everything from byte zero. `showing::frame_part` seeks
and reads a bounded window, so the allocation is the window and never the
capture's length — answering fewer bytes than were asked for is legal, and the
player asks again.

Bridge's half is a privileged scheme in the main process, which forwards the
range to Fleet and streams the answer back. **The renderer still never reaches
Fleet's port** — `docs/practices/bridge.md` is where that half is written down.

## Protocol 12.0: a step's evidence is two questions

`#777`. `ipc::EvidenceType` loses `shown`, and removing a value from a set the
wire carries is a major bump by this document's own table — so the major moves
and the minor resets.

`shown` was never a claim a Drone handed in. Every other value names a work
product the gate measures against the step's own declaration; that one meant
*Fleet, run the repository's harness*, which is an instruction. Holding both in
one field is why a step could not hand in a patch **and** be captured, and a
WorkflowDef step now says the two separately: `evidence.submitted.type` is the
claim, `evidence.captured` is the instruction, and it gates nothing.

**No DTO gains or loses a field.** `ipc::Submitted.evidence_type` still carries
what a submission was recorded as, and that is still the workflow's word rather
than the Drone's. Being captured is a fact about the frozen step and not about a
submission, so nothing on this seam had to carry it — what a capture produced
already reaches Bridge as `frames` on the step, unchanged. The break is the
narrower set alone, which is why the refusal is worth the major: a Bridge built
before this looks `shown` up in the generated vocabulary and finds a word Fleet
can no longer send.

## Protocol 13.0: a step's flags are every attempt's

`#791`. `StepDetail.flagged` held the newest attempt's gaming flags, with no
attempt on them; it now holds every attempt's, each stamped with the run that
raised it — the change `check_runs` and `judged` took at 7.0.

**The field is not new, its meaning is, and that is the major.** A Bridge built
before this reads `flagged` as where the step stands now, and handed every
attempt's flags it would draw a run that is over as the reason the step is held.
The store always kept a flag's attempt; only the detail route dropped it.

## Protocol 13.1: a spec a person picks

`#619`. `show_again` gains an optional body naming which spec to run,
`ShowAgain` gains `specs` — every spec this Job's Drones named, latest first —
and `ShownSet` gains the `spec` a press ran. Additive in all three, so the minor
moves.

**The choices are the record's own words.** A spec reaches `evidence.run` as an
argument, so the wire had to answer what stops a path leaving the worktree. It
is not validation: Fleet refuses anything that is not in the list it just sent,
and every entry in that list is a `shown_by` a Drone submitted from inside the
worktree. A directory listing or a typed path would each have needed a rule
about `..` and about absolute paths; a list has none to break.

**A press with no body is unchanged**, which is what keeps this additive in
behaviour as well as in shape: an older Bridge sends nothing and runs the last
spec a Drone named, exactly as it did at 13.0.

## Protocol 13.2: the limits a person changes

`get_limits` and `save_limits` are new routes, so the minor moves. `cpu` leaves
`admission_hold` in the same change, and that is minor too: Bridge reads the set
as opaque, and a Bridge built before this simply never sees the word again.

**A value out of range does not decode.** `SaveLimits` holds each field as a
bounded integer, so the refusal is the ordinary undecodable 400 and Fleet is
never asked. Bridge bounds the field the same way, so a person meets the range
before the wire does.

## Protocol 13.3: the moment a Drone submits

`#813`. `evidence.submitted` is a new event kind, so the minor moves. The row
had been declared since `522dac92` with no variant behind it, and the mark that
said so came off in the same change.

**It is a pointer and the decision was that it stays one.** The stream is one
bounded drop-oldest broadcast every Job shares, which is the argument that had
kept this unbuilt — and it defeats a payload-carrying event and nothing else.
So the message names the Job, the step and what the frozen step asked the work
product to be, and `get_evidence` serves the three sentences to whoever opens
the Job. `job.step_advanced` is the shape it follows.

**Published where the fact is, which is the Evidence call.** The gate notices on
its next turn; a message sent from there would be dated wrong and would say what
`job.checking` already says one message later.

## Protocol 13.4: Always allow picks a rule, not a whole command

`#834`. `CommandInFlight` and `Refusal` each gain `rules` (the leading cuts of
the command, shortest first) and `suggested_rule` (the one pre-selected), and
`AnswerCommand` gains `rule` — the one a person picked, read only where the
answer is Always allow. All three are additive: an old Bridge neither reads
`rules` nor sends `rule`, so it keeps writing the whole command as the rule,
exactly as it always has, and the minor moves rather than the major.

## Protocol 13.7: Always allow stops writing to `armada.yml`

`#836`. Fleet's own commit of the always-allowed line was itself a change on
the Job's branch, so the absolute boundary on `armada.yml`
(`crates/verification/src/forbidden.rs`) refused every later step of the Job
the allow was pressed on. Always allow now commits nothing: the rule is kept
in a table of its own, per Manifest, and granted to every Job against it the
way a declared, non-destructive Command already is.

`get_repository_allowed_commands` and `remove_repository_allowed_command` are
new routes, so the minor moves. `ipc::JobDetail` gains
`repository_allowed_commands`, additive for the same reason — every rule a
person always-allowed for the Manifest, read-only there, beside the Job's own
`allowed_commands`. **`allowed_commands` itself changes what it means, not its
shape**: a `repository`-reach row there is now one an older Fleet wrote before
13.7, kept rather than migrated, and never one this Fleet writes going
forward — an old Bridge reading it as before still reads a real historical
row, so nothing about the field's shape or presence changed under it.

## Protocol 13.24: Armada's review of a change

`ipc::JobConfidence`, additive on `get_job`'s `JobDetail` as `confidence`, is the review a person reads at the stop before merging (#903): whether Armada is confident and why, the change's areas, the tests in it, and its findings sorted into needs you, small fixes and for context. `tests.opened_because` names a test removed or loosened with no reason, and that row is `flagged`.

**Not `review`.** `JobDetail.review` is the text Fleet composes for the pull request, from 10.11. **Absent is a Job with no accepted review**, which is every Job whose steps ask for none.

## Protocol 13.25: the code a review is about, as a View

`ipc::ViewStepRow`, additive on `JobConfidence` as `view` on an area and on a finding (#904): the files it is about, as steps in the order one change forces the next. Each step names its hunk by the patch's `@@` header, with a one-sentence summary and, except on the last, what ties it to the next.

**The hunk is named, never copied.** Bridge finds it in the patch `get_job_diff` serves, so a View cannot show code the branch no longer holds. **Left out where empty**, which is every area and finding the reviewer gave no View.

## Protocol 13.29: findings a person dismissed

`ipc::DismissedRow`, additive on `JobConfidence` as `dismissed`, and `dismiss_finding`'s body `ipc::FindingDismissed` (#907). A dismissed finding leaves `needs_you`, `small_fixes` and `for_context` and is listed in `dismissed` with the reason a person gave. **Left out where empty**, which is every Job nobody dismissed anything on.

## Protocol 13.30: a person's add and drop reach the plan

`#897`. `add_task` and `drop_task` are new routes, so the minor moves —
`13.27`, `13.28` and `13.29` having each reached `main` first for unrelated
changes. A person adds a task (`title`, `detail`, `after`) or drops one with
a reason (`task`, `reason`), each kept under `store::PlanHand::Person` so the
record shows who made the change. Both answer with `ipc::WorkPlan`, the plan
the change leaves.

**Delivery follows `redirect_drone`'s rule.** With a working Drone mid-step, a
turn is injected naming the task and, for a drop, the reason — its own
`Occasion::Plan`, so the log shows it was not a redirect. At a step boundary,
or with no session, nothing is sent, because the next brief's THE PLAN is
built from the record at every spawn and carries the change already. Neither
route ever respawns a Drone to deliver itself.

Refused by name: no plan recorded (`fleet.no_plan`), a task or a place to add
after the plan does not hold (`fleet.no_such_task`), a task already `done` or
already `dropped` (`fleet.task_already_settled`, a 409 — a person's drop does
not repeat a decision already made), and a blank title or reason, on
`redirect_drone`'s reuse of `fleet.unacceptable_proposal` for a value that
cannot work.

## Protocol 13.33: a Job's review model

`JobDetail.review_model_override` and `JobDetail.review_step`, additive, and the command `set_review_model`, which takes `set_model`'s body (#903). A person chooses the model the step that writes Armada's review runs on, and on that step it beats `model_override`. `review_step` is that step's label, **absent on a workflow with no review step**, which is where Bridge draws no review model at all.

## Protocol 13.34: a pull request's CI, and what a person does about it

`PullRequestDetail.checks`, additive: what the forge's own CI came to on the pull request, as the sweep last read it, with the names of the checks that failed (#905). Two commands take no body: `rerun_failed_checks` asks the forge to start the failed runs again, and `investigate_failed_checks` sends the Job back to the step before the one that delivers with the failed checks as the next Drone's note. **Neither posts anything on the pull request.**

## Protocol 13.36: what a review finding became

`JobConfidence.followed`, additive: each For context finding a person turned into a Job queued behind this one, by its id, or into an issue, by its address (#906). Two commands: `queue_after_finding` takes the finding and proposes a Job created waiting on this one, and `file_finding_issue` takes the finding with the title and body a person confirmed and files it on the forge. **Neither posts anything on the pull request.**

## Protocol 13.37: the agent door answers about the repository a session stands in

`?manifest_id=`, optional and additive, on `list_jobs`, `list_job_board`, `list_reviews`, `get_activity_feed`, `list_alerts`, `list_drones`, `list_worktrees`, `list_servers` and `get_events_since`: absent is every repository, as before, so Bridge's All view is unchanged (#987). A named `get_events_since` counts events about that Manifest, about a Job it owns, and those naming neither, which are the machine's. `armada mcp` names the Manifest it walked to on every call to `/agent/mcp`, and the door names it on each of those routes and on the Manifest reads. Through the door, a `:job_id` another Manifest owns is refused as `fleet.job_in_another_repository`, a call naming another Manifest is refused, and `propose_job` takes its owner from the scope. Bridge's own routes are unscoped.

## Protocol 13.38: a workspace's Command runs where the root has no Manifest

`?repository=<root>` on `start_checkout_run`, `undo_checkout_run`, `list_checkout_runs`, `get_checkout_run_output` and `get_checkout_run_diff`, refused beside `?manifest_id=` as Verify's routes are (#986). `StartCheckoutRun.workspace`, optional, names a directory whose own `armada.yml` declares the Command. `CheckoutRunSheet.workspaces` lists each workspace's Commands, and `workspace` rides on `CheckoutRunUnderway` and `CheckoutRunRecord`. All additive.

## Protocol 13.40: how many of a step's Checks run at once

`LimitValues.checks_at_once` and `SaveLimits.checks_at_once`, additive: a fourth limit on `get_limits` and `save_limits`, from 1 to 8 (#284). Fleet runs a step's Checks up to it, and before starting each one after the first reads the machine against the memory and disk limits, so a short machine makes the next Check wait for a running one to finish.

## Protocol 13.43: a gate reuses a passing dry run

`CheckRun.reused_from_dry_run`, additive: when a Check's result came from the Drone's own `run_checks` instead of the gate running it again, rather than absent for a Check the gate ran itself (#1014).

## Protocol 13.44: Helm puts an approval card in front of the person

`ask_person_to_approve`, additive: a new command, `POST /jobs/:job_id/ask_person_to_approve`, answering `AskedApproval { job_id, handle }`. `agent_access = "Drafts only"`, the door offers it to a Helm session alone (#1041). It writes nothing — `Resolved` already turns a Job id that names nothing into the ordinary 404, and past that the route hands the id and handle back untouched. `approve_dispatch` stays `agent_access = "No"` for every agent; this only names which Job `HelmThread` draws a card for, and the person's own press on it is still what releases the Job.

## Protocol 13.45: a Drone's own run of the Checks, shown as it runs

`StepDetail.dry_run` and the `job.dry_run` event, additive: the Checks a Drone asked for mid-step, in `ChecksUnderway`'s shape, from their start until the Drone asks again, submits or the step ends (#1062). `checking` stays the gate's alone, and an event kind of its own keeps a Bridge that does not know it from drawing a Drone's run as the gate's. `CheckUnderway.stopped_by`, on a Drone's run alone, names the Check whose failure stopped this one before it finished, and its `produced` says so in words.

## Protocol 13.46: what a worktree's build started from

`RunSheet.seeding` and `CheckoutRunSheet.seed`, additive (#1064). `seeding` is absent where the Job's Manifest declares no `setup.seed`; otherwise it is `seeded`, with the base commit and the directories cloned, `cold`, with Fleet's sentence for why, or `unrecorded` for a worktree cut before seeding existed. `seed` is absent where the Manifest declares none; otherwise it names the directories and the Commands that warm them, and says whether the seed at the current base commit is `warm`, `warming` or `cold`.

## Protocol 13.47: a Check waiting for room says what it waits behind

`CheckUnderway.waiting_behind`, additive: on a Check still waiting, how many Checks from other work hold the machine's places while its run waits for one (#1063). `LimitValues.checks_at_once` keeps its shape and range and now counts across the machine — every Job's gate, every Drone's own run, fix drafts and proofs after a merge share it — so a gate can wait on work that is not its own. Absent is a Check waiting on nothing but its own run.

## Protocol 13.48: where a Check runs

`DeclaredCheck.runs_at`, `StepDetail.held_for_handoff` and `WorkflowStep.held_for_handoff`, additive (#849). `runs_at` is `gate` for a Check a Drone's own run never asks and `handoff` for one that runs last, on the step before handoff, once every other Check there passes; absent is everywhere. `held_for_handoff` names the handoff-only Checks a step's gate leaves to a later step, so a step that passed is not read as having run them. A handoff-only Check that was not reached records `skipped`, with its own sentence in `produced`.

## Protocol 13.51: a Check's own weight

`CheckUnderway.places`, additive beside `waiting_behind`: how many of the machine's places this Check takes, absent where it takes one (#1102). A browser suite costing more than `format` now says so where its wait is; Bridge names it only for a Check taking more than one.

## Protocol 13.52: running a stopped step's Checks again

`rerun_checks`, additive: a new command, `POST /jobs/:job_id/rerun_checks`, with no body, answering `JobSummary` (#1105). `Stuck.recourse` gains `rerun_checks`, offered on a Job at `awaiting_repair` whose stopped step failed a mechanical Check. Bridge reads `recourse` as strings, so a Bridge that predates the value draws nothing for it. The request waits for the Checks, which Fleet runs on a task of its own.

## Protocol 13.53: what Fleet resolved Helm's action authority to

`FleetHealth.helm_action_authority`, additive (#1127). `settings.helm-action-authority-tier-1-redirect-enabled-vs-read-only` resolves once when Fleet starts, and until now nothing on the wire carried the answer — Settings' "This machine" section could only describe what the setting does, not say what Fleet actually decided. `GET /health` answers it now, alongside the probes it already carried.

## Protocol 14.0: a person no longer sends the conflict back

`#1131`. `resolve_pull_request_conflict` is gone — the route, the `Commands`
method and the button that pressed it (`Grounds.tsx`, `Decide.tsx`,
`verdict.tsx`). Removing an operation is a major bump by this document's own
table, so the major moves and the minor resets.

Fleet finds the same conflict where its sweep already reads one
(`fleet::currency`) and sends the Drone back itself, as `Actor::Fleet`, once
per base — `fleet::conflict_resolution::sent_to_clear_conflicts` is what a
person's press used to reach and is now reached only from there. A Bridge
built before this offered a press that answered `fleet.route_not_found`; there
is no road left for it to hit, and nobody presses anything now.

## Protocol 14.2: a sub-dispatched Job names its parent

`JobSummary.dispatched_by`, additive: the parent Job's id alone, where `origin` is `sub_dispatched` (#1165). `sub_dispatched`'s registry sentence, `"Sub-dispatched by {dispatched_by.job_id}"`, had nothing to fill its slot with, so Job detail's facts line drew nothing for it. `dependencies` and `gate_manifests` stay off the wire — the M1 decision they were withheld alongside `dispatched_by` for — since a caller reading every row still cannot draw the DAG either would.

## Protocol 14.3: where a loop returns to

`#1149`. `WorkflowStep.verdict_routing_target` and `WorkflowStep.iteration_cap`, and `StepDetail.verdict_routing_target` beside the existing `StepDetail.pass`, all additive. A person approving a dispatch could not see that a workflow loops, because `structure: loop` only labels the edge — `verdict_routing`, in `crates/config/src/workflow.rs`, is the only place it is named, and it had never crossed. `pass.of` already carried the cap on a running Job's step; the target it points at had not, on either DTO.

**On the step that sends the work back, not the step it is sent to** — `pass`'s own rule, and the same edge. `WorkflowStep` carries both fields together: a target with no cap could not stop, and a cap on a step routing nowhere answers a question a preview never asks.

## Protocol 14.4: an abandoned step's restart names a new trigger

`#1034`. `EscalationTrigger` gains `drone_gone`, the step-level trigger a
person's restart writes over a step whose Drone left before anybody acted —
`drone_killed`'s and `run_ended`'s third sibling. **Minor, on `queued_reason`'s
precedent**: `escalation_reason` carries no `wire_enum!` in `crates/ipc`, so
Bridge reads it as an opaque string through the generated vocabulary rather
than matching on it, and a new value is additive while nothing branches on it.

## Protocol 14.5: when each plan task was being worked

`#1185`. `PlanTask.working_windows`, additive and left out where empty: each
stretch a task was marked `working`, as `WorkingWindow { entered, left? }`,
oldest first. Fleet folds it from the plan's history — a move into `working`
opens one, any move out (`open`, `done`, `dropped`) closes it, and a new
recording starts every task with none. `left` is absent while the task is
still working. Bridge places a turn in the task whose window holds its
instant, so the Working area can group a step's activity by task. **A claim,
like the state it comes from**: a Drone that never calls `update_task` sends no
windows, and its work belongs to no task.

## Protocol 14.7: Helm's act on a Studio is its own event

`#1288`. `studio.helm_acted`, a new event kind, additive: published after the
`studio.changed` a write publishes, only where the door placed the call in a
Helm session, carrying `StudioHelmActed { studio_id, manifest_id, act, at }`
with `act` one of `added_node { node_id }`, `proposed_edge { edge_id }` or
`named { name }`. A person's act on a Studio publishes `studio.changed` alone,
so Helm's act is told apart by kind — `docs/concepts/helm.md`, *Audit trail*.

`StudioNode.added_by`, `StudioEdge.added_by` and `Studio.named_by`, additive
and left out where absent: `person` or `helm`, kept by store migration V78. A
row from before V78 has none, since Helm could already act under V77 and a
default would name an author nobody recorded.

`get_checkout_run_sheet`, `list_checkout_runs` and `get_checkout_run_output`
move from `No` to `Helm only`. **That half moves no number**: `agent_access`
decides what the agent door offers, which is not the Fleet/Bridge seam, and no
message either side parses changed.

## Protocol 14.8: a scout, and what its Finding read

`#1292`. Three commands and a Finding's fields, all additive. `ask_scout` (`Bridge only`) adds a Finding Gathering and starts its scout; `start_scout` (`Helm only`) starts a Finding already Proposed; `stop_scout` (`Bridge only`) is the stop on its node. A Finding's content keeps `asked` and gains `checkout` (`commit`, `uncommitted`), `read`, `searched`, `learned` and `ended` (`outcome` of `answered`, `stopped` or `failed` with `why`, and `cost_micros`), each left out until the scout records it — so a Proposed Finding is on the wire exactly as it was at 14.6.

**`cost_micros` is absent, never nought, where no cost was reported**: a scout whose group had to be ended rather than interrupted reports none, spike 017. `outcome` is a serde tag rather than a `wire_enum!`, for the node's own `kind`'s reason: it is the field the rest hang off. A Finding added through `add_studio_node` carrying any of the new fields is refused as `fleet.studio_finding_is_the_scouts`.

## Protocol 14.9: Helm is told which Studio a person has open

`#1287`. `HelmScreen` gains `studio`, the Studios surface, and `HelmContext` gains `studio` and
`node`, both optional and left out where empty: the Studio open on that surface, and the node
selected on its whiteboard, only beside its Studio. Fleet's line to the session names both by id,
so Helm can read the Studio with `get_studio` rather than guess one. **Minor, though Fleet matches
on the screen**: Bridge sends `studio` only to a Fleet at 14.9 or later, because a Fleet behind
Bridge is refused before an ask is ever sent.

## Protocol 14.10: counts on the live file list

`#1187`. `ChangedFile.lines`, additive and left out where absent: what a file
gained and lost, on `job.files_changed` only. Counting is the walk that renders
the patch, so Fleet counts on a due reading only once the Drone has made no call
since the reading before, something moved since the last count, and ten seconds
have passed since it. A reading between two counts carries the last count for
each file still listed, and none for a file that arrived since. Absent is not
zero, as on `TouchedFile.lines`.

## Protocol 14.11: Studio capture, and what a Note keeps of it

`#1290`. One command and one optional field, both additive. `capture_studio_note` (`agent_access`
`No`) puts a Note on a Studio where a person pointed in Bridge. A `note` node's content keeps
`said` and gains `capture`, left out on a Note that was typed rather than pointed — so a Note from
before this is on the wire exactly as it was at 14.6.

`capture` carries the development annotation layer's own fields — `component`, `owners`,
`selector`, `element`, `screen`, `layer`, `location`, `bounds` and `window` — and four the layer
does not record: `styles`, `markup`, `source` and `frame`. **`source` is absent, never guessed**:
React 19 fibers carry no `_debugSource`, so Bridge sends a path only where the build gives it one.

**The frame crosses as a staged file and reads back as a kept one.** The request's `frame` is
`staged_path`, `width` and `height` — the PNG Bridge's main process wrote where `stage_attachment`
writes one. Fleet copies it under `<machine>/studios/<studio_id>/` and the Note's `capture.frame`
names `filename`, `byte_size`, `width` and `height`. Nothing about where Bridge staged it reaches
a client, and no image crosses in a `studio.changed`. A staged frame over 4 MiB is
`fleet.studio_frame_too_large`, and one Fleet cannot read is `fleet.studio_frame_unreadable`.

## Protocol 14.12: a Note's frame, read back

`#1352`. One route, additive: `GET /studios/:studio_id/frames/:node_id` answers the picture a Note
kept as the file itself, the way `get_frame` answers a step's. 14.11 wrote the frame and gave a
client no way to read it.

**One path segment where a step's frame takes two.** A Studio keeps one frame per node, under the
node's own id, so the node names the file — and the name is read off the node's `capture.frame`
before anything is opened, which is what keeps a caller's text off a path. A node that is not on
the Studio is `fleet.no_such_studio_node`, a node that kept no frame is
`fleet.studio_frame_not_kept`, and a file that will not open is `fleet.studio_frame_unreadable`.

`agent_access` is `Bridge only`, where `get_frame` is `Yes`: a Note's frame is a photograph of the
window a person was working in, not of a harness's own page. **The bytes reach Bridge's renderer
over the preload and become a `blob:`** — the CSP's `img-src 'self' blob:` is unchanged, and no
scheme was added to it.

## Other things specific to this seam

**Bridge finds Fleet through a runtime file, not a fixed port.** The file
carries port, pid, and protocol version, and Bridge verifies the pid is still
alive before treating the port as live — a stale runtime file and a genuinely
unreachable Fleet look identical over a bare connection timeout, and the pid
check is what tells them apart. Any change to the runtime file's shape is a
protocol-adjacent change even though it never touches `ipc`: it's still a
contract two independently-versioned binaries agree on ahead of any
connection. Treat it with the same "what does an old reader do with an
unrecognized field" discipline as the DTOs.

**One route on the listener is not on this seam.** `/mcp` serves the Evidence
tool to a Drone — the only way a Job's work is ever reported. It shares the
port because a Drone reaches Fleet the same way Bridge does, and it shares
nothing else: the peer is a process Fleet itself spawned, the vocabulary is
MCP's rather than `ipc`'s DTOs, and the version negotiated is the MCP revision
the client asks for rather than `protocol-version.toml`'s. So it is
deliberately absent from `operations.toml` and from `SERVED`, and a row added
for either would claim Bridge can call it. It also means the rule below does
not cover it: the address is written into a Drone's `mcp.json` from `api`'s own
constant, and that shared value is what stands between a typo and a Drone that
can never report.

**The rule that reads `operations.toml` runs both ways now.** One direction
fails on a route serving a name the inventory does not have; the other fails on
a name the inventory has and nothing serves. The second carries an allowance,
and every entry in it states a reason the gate prints — a list of exemptions
with no sentence each is the silent default the column was given reasons to
remove.

**A second route on the listener is not on this seam either, and it is this
seam spoken differently.** `/agent/mcp` is the agent's door: an MCP client
reaches it, and every tool on it is one row of `operations.toml` served at the
route `SERVED` already names, so there is no second implementation to drift.
What it adds over the HTTP surface is a cap — a tool answer over 64 KiB is cut,
says so, and names the route that serves it whole — and a scope: every answer
is inside one Manifest, and the handshake says which. It answers 405 to `GET`
and `DELETE` for `/mcp`'s reason, so it adds nothing to the risk above.

**Who opens it is the repository somebody is standing in.** `armada mcp` is the
relay a repository's own `.mcp.json` names: it reads `fleet.json` for the port,
refuses on `Stale { PidHeldByAnother }` rather than connecting to a port another
process now holds, and resolves its own working directory to a Manifest — never
a request field, for the reason a Job id is not one on a Drone's tools. A
session started below a repository root walks up to it, and says at its
handshake which root it settled on. The walk ends at a repository that has no
Manifest of its own, and at the home directory, so it can reach neither a parent
repository nor an `armada.yml` sitting above every project on the machine.
A session that resolves to no Manifest, or to one this Fleet is not serving, is
answered rather than dropped: the handshake succeeds and carries the reason, and
`tools/list` is empty. **None of it is authentication** — the bind is loopback
with nothing in front of it, so this selects a Manifest and grants nothing.

**The route table is hand-written, and that's an accepted cost, not an
oversight.** A typo in a route path is a runtime 404/500, not a compile error,
on both the main protocol and the lifeboat. That trade was made deliberately
in exchange for not carrying codegen where it isn't earning its keep — see
gRPC's rejection above. It means route changes need a `curl` or integration
check in the same change, because the type system will not catch this class
of mistake for you.

## Protocol 14.13: what is on a Studio becomes work

`#1291`. Six commands and one optional field, all additive.
`group_studio_nodes` (`Bridge only`) accepts several nodes as one Cluster or
reads them in order as one Outline, with a `produced` edge from each in the
order given; `defer_on_studio` (`Bridge only`) adds a Deferral with an accepted
`blocks` edge to what it holds up; `write_up_studio_node` (`Helm only`) adds an
Issue draft; `edit_studio_draft` (`Bridge only`) replaces that draft's title and
body; `settle_contradiction` (`Bridge only`) ends a Contradiction as
`not_a_problem` or `resolved_here` with its answer; `dispatch_studio_draft`
(`Helm only`) sends the draft's text through the Job proposer and answers with
the Studio carrying a Job node per Job, each on a `produced` edge from the
draft.

A Contradiction's content gains `answer`, **absent unless a person ended it as
*Resolved here***, so a node written before this is on the wire exactly as it
was at 14.6. `HelmStudioAct` gains `wrote_up { from, node_id }` and
`dispatched { from, node_ids }`, the two acts Helm takes on a person's ask —
`docs/concepts/studio.md` publishes every act of Helm's, not only the unasked
ones.

**No new node kind, no new state and no migration.** Every kind a rung makes —
Cluster, Deferral, Issue draft, Outline, Job — and every state it sets was
already in `core-model` and in V77's `CHECK`, because `#1285` wrote the whole
vocabulary down. What was missing was the calls.

**Nothing here reaches a forge.** No operation files an issue, and dispatch
carries the draft's own text with nothing to point at: filing is optional and a
person's own act.

A Job dispatched from a Studio takes `manual` or `helm_drafted` for its
`origin`, by who pressed it, rather than the `auto_detected` every other request
through the proposer takes — **a value already on the wire, so it moves no
number.** What it changes is what a row says: *Found by Fleet* names work
Armada noticed by itself, and a draft somebody wrote up and sent is neither.

## Protocol 14.14: a person names a Studio and puts a node on one

`#1364`. No shape moves. `rename_studio` and `add_studio_node` are reached from Bridge as well as
Helm — the `agent_access` column says which *agents* a route is offered to, and a person's own
call was never narrowed by it — and `add_studio_node` gains one refusal, `fleet.studio_node_not_a_persons`,
for a kind a person may not mint by hand. A person adds a `note`, a `link` or a `sketch`; every
other kind is made by the act that earns it, so the person's refusal and Helm's
`fleet.studio_node_not_helms` meet over the kinds neither side adds.

**Minor because a refusal code added is additive**, the way one removed is: an older Bridge reads
an unknown code as a refusal with the message beside it, which is what the error contract promises.
Bridge's own halves of both calls are new capabilities on the preload bridge and cross no wire of
their own.

## Protocol 14.15: a line of a person's own on a Link

`StudioNodeContent::Link` gains `said`, optional: the line a person wrote beside the address saying
why they kept it, and `edit_studio_link` is the operation that changes it afterwards. `#1378`.

**Additive on both counts.** The field is left out where there is none, which is exactly the shape
every Link written before it already has, so an older Bridge reads a Link as it always did. The new
route carries the line and never the address — a Link never stops being its address — and a blank
line clears it rather than being refused. A kind that is not a Link is refused as
`fleet.studio_not_a_link`, a code added the way every other refusal here was.
## Protocol 14.16: Helm writes a file in the checkout

`#1373`. `helm.changed_checkout`, a new event kind, additive: published as a Helm session's own
stream says it wrote a file, carrying `HelmChangedCheckout { manifest_id, tool, path, at }`.
Helm edits the repository's checkout directly on a person's ask, with no worktree and no Job
around the change, so this is what lets somebody who finds a file changed see that Helm changed
it — `docs/concepts/helm.md`, *Audit trail*, and the rule `studio.helm_acted` already follows.

**It names writes Armada can name, not every change Helm caused.** `tool` is one of the built-ins
that edits a file, so the path is known; a shell line may also have written something and nothing
in the stream says whether it did. Those calls stay on the conversation's own socket, where they
already were.

## Protocol 14.17: what a Link's address names on the forge

`StudioNodeContent::Link` gains `forge`, optional: `issue`, `pull_request` or `milestone`, and
absent where the address names nothing on the forge. `#1379`.

**Read off the address by Fleet on every Studio it sends, and never kept on the record.** Which
host is the forge is `crates/adapters`' to know — `verify-foundations` refuses the vendor's name
anywhere else, Bridge and `crates/ipc` included — so a rule about issue links could not be written
in TypeScript at all. This field is how Bridge knows to offer Dispatch on a Link naming an issue
without reading one.

`dispatch_studio_draft` takes such a Link as well as an Issue draft, and sends the address as the
request. **No shape moves on that route**: the request already carried `node_id` and `position`,
the gate is the same gate, and the Job node lands with a `produced` edge from the node it came
from either way. A Link naming anything else is refused as `fleet.studio_not_a_draft`, the code
that route already had.

**Additive on both counts.** An older Bridge reads a Link with no `forge` as the Link it always
read, and an older Fleet is refused by the skew rule as it always was.

**Superseded at 14.18**, which makes what an address names the node's own kind and drops this
field.

## Protocol 14.18: an Issue, a Pull request and an Epic are node kinds

`StudioNodeContent` gains `issue`, `pull_request` and `epic`, and `Link` loses `forge`. `#1394`.

Each of the three carries `address`, `number` and `said`, and a `title` absent until the node is
read in. An Issue and a Pull request carry `state` — `open`, `closed` or `merged` — and an Epic
carries `read_in`, `{ issues, total }`, how many of its issues are on the Studio of how many it
holds. Every field but `address` and `number` is optional and left out rather than sent as null.

**The kind is the concept and the adapter decides it.** `adapters::forge_node` reads an address
once, when the node is made, and answers with the node's content. Nothing reads an address again:
`Studio::of` no longer takes a classifier, and `dispatch_studio_draft` offers the three by kind.
A Link is what no adapter recognised — a board, a page, a document, a session — and dispatches
nothing.

**`add_studio_node` still takes a Link, and Fleet writes what it is.** Bridge cannot read an
address, so the seam carries the paste and not the kind; `StudioNodeByHand` is unchanged.

**Additive by 14.7's reading, which added `finding` the same way.** `forge` goes with nothing that
carries one: a Link whose address names something on the forge is converted on the boot that
applies store V79, so no message an older Bridge parses stops parsing the same way. An older
Bridge meeting one of the three leaves it off the whiteboard rather than failing, which is
`whiteboardEdges`' rule for an unknown edge and is what `cardOf` gained here.

## Protocol 14.19: a person answers a call Helm was refused

`#1389`. Two new event kinds, three new routes and a DTO family, all additive. `helm.asking_to_run`
carries `HelmAskingToRun { waiting: HelmCallInFlight }` when a Helm session reaches for something
the person's own agent settings do not cover; `helm.call_answered` carries `HelmCallAnswered {
call, manifest_id, tool, detail, rule, settled, at }` when the ask ends, whoever ended it. The
routes are `POST /helm/permission` (the agent door's own permission tool, reached by the CLI and
never by a model), `GET /helm/calls` and `POST /helm/calls/answer`.

**The ask carries no `job_id`, and that is what makes it a new type rather than a
`CommandInFlight`.** Every field of that one is about a Job — `step_id`, `allow_for_job`, a rule
written into `armada.yml` on the Job's branch — and a Helm call has none: nothing is queued, no
step is running, and what waits is one process inside one tool call. `HelmCallAnswer` is its own
closed set for the same reason, and a surface matches on it to pick controls, so a fourth value in
it is a major bump the way `WhenBlocked`'s third was.

## Protocol 15.0: a plan task carries its files and its evidence

`#1421`. `PlanTask` gains `scope`, `expects` and `shown`, and its `detail` is renamed `note`.
`AddTask` moves the same way. **The rename is what makes this a major**, and it is the whole of
what is not additive: an older Bridge reads `note` as absent and draws a task with no note at all.

A planning step already recorded which files each task touches — it wrote them into `detail` as
prose, and the step after it re-derived them by searching. `scope` is that list as a list, so the
next Drone starts from it. `expects` is what the planner says should prove the task and `shown` is
what the work says did; they are kept apart rather than reconciled, because the two disagreeing is
the fact worth seeing.

**`shown` is written by a later `update_task`, not with the recording.** It rides on the change, so
a task reopened and finished again keeps what its first pass showed.

## Protocol 14.20: an Epic read-in asks which of its issues to take

`#1405`. `ReadInLink` gains `take`, `everything` or `open`, and `EpicRead` gains `took`, `left_out`,
`kept` and `laid_out_from`. All additive, and `take` is asked on an Epic alone: every other kind has
one thing to read and nothing to ask about.

**Reading an Epic in again with the other answer widens or narrows what is on the Studio.** Fleet
makes the Issue nodes the answer wants and are not there, and takes back the ones it made that the
answer no longer wants — except any a person has since worked on, which the Epic counts as `kept`.

**An older peer is read as taking everything**, which is what reading one in used to do, so a Fleet
ahead of Bridge sends `took` and a Bridge behind it ignores it. An Epic read in before this version
carries no `took`, and is drawn saying nothing about an answer nobody gave it rather than claiming
one — which is the same rule `title` and `state` already follow at 14.18.

`laid_out_from` is Fleet's own bookkeeping on the wire: the corner of the block an Epic's issues sit
in, so a widening fills that block's gaps and a node dragged out of it is readable as dragged.

## Protocol 16.0: one delete on a Studio's nodes, one node or eighteen

`#1411`. `remove_studio_node` is gone — the route, the DTO, the `Studios` method, the store write
and the capability Bridge reached it by. `remove_studio_nodes` replaces it, `POST
/studios/:studio_id/remove_nodes` carrying `RemoveStudioNodes { node_ids }`, `Bridge only`.
**Removing an operation is a major bump by this document's own table**, as at 14.0, so the major
moves and the minor resets.

**This was written as 15.0 and is 16.0**, because 15.0 landed underneath it while the branch was
open. Both files read `major = 15, minor = 0`, so git merged them clean and the collision was
invisible — two changes claiming one version, which is the failure this file's own numbering
exists against. A version taken on a branch is a claim about the base it was taken from, and it is
re-read at every merge of `main`.

**Two routes for one act is two paths that drift, and these had.** The single-node write left a
captured Note's frame on disk; the selection write deletes it. Retiring the first closes that leak
rather than writing it down. A Bridge built before this presses a route that answers
`fleet.route_not_found`, which the major is what stops it reaching.

**All of them or none is the store's transaction, not the caller's care.** Every name is checked
before anything is deleted, so a selection carrying one name the Studio does not hold refuses with
every node still on it — including the Studio's own `touched_at`, which rolls back with the rest.
A name given twice removes that node once. A call naming no node at all is refused as
`fleet.studio_no_nodes_named` rather than taken as a write that does nothing.

**The frames go after the write, never before it.** A Note's picture is a file beside the records
and the record is what names it, so the file is deleted once the row that named it is gone. A
refused write leaves every picture where the Note that keeps it can still draw it.

## Open questions

Naming these rather than deciding them, per this document's brief:

- **[protocol-codegen]** What generates the TypeScript from `ipc`. Hand-rolled build script,
  `ts-rs`, `specta`, something else — not decided. Whatever it is, it must not
  reach `core-model` or `adapter-traits` (their `cargo tree` is a gate rule:
  no codegen framework belongs under either).
- **[verify-protocol-task]** What checks the rest of the generated half. The
  version pair is held by a `verify-foundations` rule, which is the part that
  shipped broken; the DTO types are generated by nothing, so there is no
  candidate output to compare the checked-in ones against, and no rule refuses
  a version literal spelled outside the generated file. Whether the remainder
  is a rule in `verify-foundations` or a `verify-protocol` task of its own
  follows from what `[protocol-codegen]` decides, and neither is decided.
- **[broadcast-capacity]** The bounded broadcast channel's capacity, and whether it's one number
  for all event types or tuned per event type.
- **[lifeboat-router]** Whether the lifeboat's four routes live inside the
  same `axum` `Router` as the main protocol or a separate one. Either can satisfy "no shared
  dependency with the versioned protocol"; which one hasn't been decided.
