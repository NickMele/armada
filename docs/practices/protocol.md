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

`protocol-version.toml`, at the repo root, holds one number:

```toml
version = 1
```

That file is read on both sides, but not the same way:

- **Rust** reads it at compile time. `crates/ipc/build.rs` parses it and emits
  a `PROTOCOL_VERSION` constant that the rest of the Rust workspace compiles
  against. This half is self-correcting by construction — `build.rs` runs on
  every `cargo build`, so the embedded constant cannot go stale relative to the
  file. There is no step to forget here.
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
hand because it was faster. `cargo xtask verify-protocol` exists to make that
drift a build failure instead of a runtime surprise. It checks two things:

1. The checked-in generated TypeScript matches what codegen would produce from
   the current `ipc` source, right now.
2. Nothing outside the generated file hard-codes the protocol version as a
   literal.

That second check exists because of a violation already in the tree today:
`apps/desktop/src/preload/index.ts` currently does this —

```ts
contextBridge.exposeInMainWorld('armada', {
  protocolVersion: (): number => 1,
})
```

That `1` is a hand copy of `protocol-version.toml`'s `version`, typed by a
person, with no mechanism forcing it to move when the source file does. It is
exactly the drift `protocol-version.toml` was created to make impossible, and
it is currently possible anyway because the generated-types pipeline it's
supposed to read from doesn't exist yet. When `ipc`'s codegen lands, this line
needs to import the generated constant instead of restating it — that's a
required follow-up, not a someday-nice-to-have.

**Contributor workflow, in order:**

1. Change the DTOs in `crates/ipc` (add a field, add a variant, whatever the
   change is).
2. If the change is wire-breaking (see the minor/major table below), bump
   `version` in `protocol-version.toml`.
3. Run the codegen step to regenerate the TypeScript types and version mirror
   under `packages/`. *(The exact command isn't decided yet — see Open
   Questions. Whatever it's called, it runs from the `ipc` source, not by
   hand-editing the `.ts` output.)*
4. Run `cargo xtask verify-protocol`. It fails if step 3 was skipped or done
   against a stale build, or if anything still hard-codes the version.
5. Commit the `ipc` source change and the regenerated files in the same
   change. A generated-file diff with no corresponding source diff, or vice
   versa, is the thing review should bounce.

## DTOs, not domain types

`WireError` is a DTO like any other, and the Error Contract in Notion
(https://app.notion.com/p/3c7173a35eb98126ad2bf236d9080892) is what specifies it —
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

## The v0 lifeboat

When the version check fails outright — major skew, or a Fleet too old or too
new to speak the current protocol at all — Bridge doesn't get nothing. It gets
four routes that don't depend on version agreement:

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

**The route table is hand-written, and that's an accepted cost, not an
oversight.** A typo in a route path is a runtime 404/500, not a compile error,
on both the main protocol and the lifeboat. That trade was made deliberately
in exchange for not carrying codegen where it isn't earning its keep — see
gRPC's rejection above. It means route changes need a `curl` or integration
check in the same change, because the type system will not catch this class
of mistake for you.

## Open questions

Naming these rather than deciding them, per this document's brief:

- **What generates the TypeScript from `ipc`.** Hand-rolled build script,
  `ts-rs`, `specta`, something else — not decided. Whatever it is, it must not
  reach `core-model` or `adapter-traits` (their `cargo tree` is a gate rule:
  no codegen framework belongs under either).
- **How `cargo xtask verify-protocol` gets wired into `xtask`.** Today
  `xtask/src/main.rs` only implements `verify-foundations`; there is no
  `verify-protocol` task and no `ipc` crate for it to check. It needs to exist
  before anything above is enforceable rather than aspirational.
- **The bounded broadcast channel's capacity**, and whether it's one number
  for all event types or tuned per event type.
- **Whether the lifeboat's four routes live inside the same `axum` `Router`
  as the main protocol or a separate one.** Either can satisfy "no shared
  dependency with the versioned protocol"; which one hasn't been decided.
