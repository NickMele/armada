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

The three people get wrong most often: widening an enum "because it's just
adding cases," making a field `Option<T>` "to be safe," and renaming a variant
"for clarity." All three feel non-breaking from inside the change and are not.
If you catch yourself writing "this shouldn't break anything, it's just
adding/loosening X" — that sentence is the tell. Stop and check whether the
other side's code has an exhaustive match, a presence assumption, or a name
lookup anywhere near the thing you're touching.

**The caveat row has exactly one instance, and it is deliberate.**
`FleetCapacity.held_by` — which one of the concurrency bound, CPU, memory or
disk is stopping the next Drone — is a `String` on the wire rather than a
`wire_enum!`, and `crates/ipc/src/capacity.rs` is where that is argued. Fleet is
the only writer, Bridge looks the value up in the generated vocabulary rather
than matching on it, and that map already answers `undefined` for a key it does
not hold. So a fifth reason is a `core-model` variant, a row in
`enum-verbs.toml` and a codegen run, and it moves neither number here.

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
| Same | The majors and the minors agree | Connects. The status bar says nothing about versions |
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
reach. It goes in the status bar beside the running dot, as advice on a healthy
connection, and **not** as a failure notice: a minor gap Bridge can survive is
not a fault, and drawing it as one tells somebody something is broken when it is
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
process now holds, and resolves its own working directory to a Manifest —
never a request field, for the reason a Job id is not one on a Drone's tools.
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
