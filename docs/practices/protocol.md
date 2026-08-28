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
hand because it was faster. `cargo xtask verify-protocol` is what would make
that drift a build failure instead of a runtime surprise. **It does not exist
yet** — see `[verify-protocol-task]` below — so what follows is what it is for,
and none of it is enforced today. It would check two things:

1. The checked-in generated TypeScript matches what codegen would produce from
   the current `ipc` source, right now.
2. Nothing outside the generated file hard-codes the protocol version as a
   literal.

That second check exists because of a violation this document was written
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
   `apps/desktop/src/shared/generated/` from `protocol-version.toml` and
   `crates/core-model/domain/`, and prints one line per generated file plus any
   registry row it could not render. **It emits the version mirror and the
   enum vocabulary, not the DTO types** — nothing generates those from the
   `ipc` source yet, so a shape change is still hand-mirrored on the TS side
   and that is the gap `[protocol-codegen]` names.
4. **There is no check to run.** `cargo xtask verify-protocol` does not exist
   — `[verify-protocol-task]` below — so until it does, step 3 is verified by
   reading: the generated file's version matches `protocol-version.toml`, and
   nothing hard-codes it. `xtask` offers `verify-foundations`, `verify-tokens`,
   `verify-docs` and `verify-roadmap`, and none of them checks this.
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
working. `apps/desktop/src/renderer/src/fleet.ts` carries the sentences and
`apps/desktop/src/shared/version.ts` carries the rule; `crates/ipc/src/version.rs`
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

The fix direction, not yet built: a **bounded broadcast channel with
drop-oldest**, so a slow consumer loses old events instead of causing Fleet's
memory to grow without limit. Bounded-and-lossy only works if the client knows
it happened, so the drop has to come with a message: *you missed N events,
resync.* The reason this matters as a protocol concern and not just a
performance one is what happens without it — a reconnecting Bridge that
silently believes its event log is complete will render a Job Board that's
quietly wrong, and "quietly wrong" is worse here than "visibly stale," because
nothing on screen tells the person to distrust it.

If your change touches the event stream, say explicitly whether it makes this
better (bounds something, adds a resync signal) or worse (adds another
unbounded queue, another place assuming delivery is complete).

## The second socket: one Job's turns

`GET /jobs/:job_id/observe` is a WebSocket upgrade, and it is the one query in
`operations.toml` whose transport is the socket. It answers with the turns a
Job's Drones have already taken and then continues with the ones that follow,
so joining a Job already running takes one connection rather than a history
call and a subscription that have to be stitched together.

**Who reads it.** A person, through Bridge, on the machine Fleet is running on.
`helm_access` is `No`: a Drone's whole transcript streamed into a Helm session
stays in that session for the rest of it, and Helm has `get_drone` for the
snapshot.

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
- **[verify-protocol-task]** How `cargo xtask verify-protocol` gets wired into `xtask`. Today `xtask`
  implements `verify-foundations`, `verify-tokens`, `verify-docs` and
  `verify-roadmap`; there is no `verify-protocol` task. The `ipc` crate now
  exists and is generated-then-committed, so the thing to check is there and the
  check is not: the version was bumped four times on 2026-08-28 and mirrored by
  hand each time. It needs to exist before anything above is enforceable rather
  than aspirational.
- **[broadcast-capacity]** The bounded broadcast channel's capacity, and whether it's one number
  for all event types or tuned per event type.
- **[lifeboat-router]** Whether the lifeboat's four routes live inside the
  same `axum` `Router` as the main protocol or a separate one. Either can satisfy "no shared
  dependency with the versioned protocol"; which one hasn't been decided.
