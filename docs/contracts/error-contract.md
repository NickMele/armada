# The Error Contract

**Kind:** contract. **Governs:** every failure Armada represents, transports
or displays — the ArmadaError code manifest, the wire envelope, and how
Bridge renders one. The v0 lifeboat is explicitly outside it. Read before
touching `core-model`'s `ArmadaError`, `ipc`'s wire types, or how Bridge
renders one.

**Purpose:** what an error carries, how it crosses the Rust and TypeScript
halves, and what a person sees when one reaches them. **The shape, not the
handling of any particular failure.** Nothing here says what to do about a
lost worktree; it says what a lost worktree looks like by the time it
reaches somebody.

**Parent:** the [Design System — UI & Voice](design-system.md) contract
governs how the message reads once it lands. This page governs what the
message is made of, and may not contradict it.

**Why it is separate:** an error crosses four layers and three languages.
Splitting the rules across the Rust practice, the Bridge practice and the
protocol page is how the four ends stop agreeing.

---

## The path a failure takes

Four stages, and something is lost at each one on purpose.

| Stage | Lives in | What it holds |
| --- | --- | --- |
| A leaf error is raised | any crate | A typed enum variant, beside the code that raised it |
| It wraps | `core-model` | `ArmadaError` — a real cause chain, `Box<dyn Error>`, never a formatted string |
| It crosses the wire | `ipc` | `WireError` — the chain flattened to an ordered array of strings |
| It renders | Bridge | A code looked up, or a message fallen back to |

The two that matter are the boundary crossings. The one that costs is
`From<ArmadaError>`: a traversable cause chain becomes an array of strings,
because a trait object cannot cross a wire. **Everything downstream of that
point reads strings.**

---

## What is settled

### Shape

Per-crate typed leaf enums wrap into one shared `ArmadaError` in
`core-model`, reusing the Log Envelope's `Ulid`, `FieldValue` and
`Timestamp` rather than inventing a parallel vocabulary.

Correlation attaches to **the capability type**, not to function
signatures. A `DroneVcs` already holds the ids, so `.wrap()` picks them up.
No function grows a parameter for logging's sake — the same move as
`Secret<T>`: the right thing happens because the wrong thing is not
reachable.

Structured fields, not prose — the values that failed, not only the kind of
failure. `FieldValue` has no variant that can hold a `Secret<T>`, so a
credential cannot be put into one without it failing to compile.

### Codes

**Declaration stays beside the variant that raises it.** The set is closed
by collection, not by authorship: `cargo xtask verify-error-codes` walks
the workspace, fails on a duplicate, and emits a checked-in manifest.

A central registry puts every code far from the failure it names and turns
adding one into a merge conflict. Scanning gets the closure without the
distance — the same shape the vendor-literal rule already uses.

### A code's meaning

**The meaning travels in the manifest.** Each code's one-line meaning is
lifted from a doc comment beside its declaration into the generated,
checked-in manifest. Changing what a code means therefore changes a
tracked file, and shows up in a diff and in review like any other change.

This closes the only failure here that every other check misses. A code's
meaning can shift under a stable name with byte-identical wire, an exact
version match and every gate green — and the first person to notice is the
one the wrong copy misled. Every other kind of drift in this contract has a
mechanism; this one had a sentence.

It also settles where this page sits: **the manifest is the record, and
this page is the explanation.** Two alternatives rejected — a stated
convention against repurposing, honest about being unenforceable; and
leaving it until a code first needs to change, which decides it under
pressure.

### The wire

`ipc::WireError` carries `code`, `message`, `run_id`, `fields` and `chain`
always; `job_id`, `drone_id` and `step_id` when they apply. **It does not
carry `level` or `component` — it is an error, not a log line.**

The `From` conversion is where redaction becomes a visible step. A domain
type on the wire is a redaction decision nobody made.

### Who mints what

**`run_id` names the emitting process, not Fleet's.** Each process mints
its own at start, so an error raised inside Bridge before it has reached
Fleet carries a real id instead of nothing.

The rule it was confused with is narrower and stands: **Fleet is the sole
authority for `job_id` and `drone_id`**, because those name records Fleet
owns, and an id invented elsewhere joins to nothing. `run_id` names an
emitter, not a record.

This also closes a question the Log Envelope was carrying from the other
end — whether `run_id` belonged on Drone lines at all, given a Drone
outlives a Fleet restart under `setsid`. One field, both problems. The cost
is that a restart is now Fleet's `run_id` changing rather than any of
them.

### Errors under backpressure

**No exemption.** An error is an ordinary event on the stream and may be
dropped like any other — but it is written to the durable record **before**
it is broadcast, so what a drop costs is the speed of noticing, never the
fact.

Exempting them sounds safer and is worse. The scenario that overruns the
channel is several Drones producing at Drone speed into a minimised
Bridge, and a failure storm is exactly when error events multiply — so the
exempt class would be the one most likely to breach the bound. An
unbounded exemption is an unbounded queue with a nicer name.

It would also spoil the one signal that makes bounded-and-lossy safe. *You
missed N events, resync* is checkable. *You missed N events, none of which
were errors* needs per-class accounting and invites a reader to trust a
partial view — the quietly-wrong failure the resync message exists to
prevent. The guarantee therefore sits in the store and the resync path, not
in a delivery promise: after a drop, Bridge re-reads rather than patching
what it already has.

### Two mechanisms, on purpose

Fleet does **not** mirror correlation onto errors as a second copy of what
its spans already carry. A log line gets its ids from the span it is
inside; an error carries them as data. Both are correct, because they
travel differently.

A log line never leaves the process where the span is live, so ambient
context is enough. An error does leave — over the wire, into Bridge, where
no span of Fleet's exists, and reading ambient context there would read
nothing.

The failure this could produce is the two disagreeing: an error stamped
with one Job's ids inside a span opened for another. It cannot happen **by
construction rather than by discipline**, because both draw from the same
scoped value — the capability type is the thing scoped to the Job, and the
span was opened from it.

### The derive macro

**Use it.** Hand-rolling the error impls would be roughly fifteen
mechanical lines per leaf enum across a dozen crates, and mechanical code
nobody reads is where drift lives.

The dependency rule it appeared to violate is narrower than it reads:
`core-model` and `adapter-traits` must pull no `tokio`, `git2` or `reqwest`
— runtimes and I/O, the things that become everyone's problem because every
crate depends on these two. A derive macro imposes no runtime and appears
in nobody's shipped binary. Both crates are `no_std`, which the macro
supports.

The real cost is compile time: a proc macro under the crate everything
depends on lands in every build, and v1's unsolved pain was a four-minute
cold rebuild. **That is a reason to measure it, not to write two hundred
lines of boilerplate first.**

### Where the code check lives

`verify-error-codes` stands alone rather than folding into the protocol
check.

Folding would couple it to a toolchain it does not need: the protocol
check requires the TypeScript codegen chain, so a contributor adding an
error code in Rust would need the whole front end installed to find out
they had duplicated a string. And the failure would read *protocol is
stale* when the actual fault is two variants claiming one code. The cost is
one more command in CI. **A check that names its own failure is worth more
than a shorter list of checks.**

### Version skew

Adding a code is **minor**. Removing one is **also minor** — a deliberate
departure from the general rule that removing anything is major.

Bridge never matches codes exhaustively; it looks one up or falls back. A
code it has never heard of and a code that has been withdrawn are the same
event to it, and both render.

---

## What the wire always carries

Bridge falls back to these when it meets an unknown code — which is every
code today, since the set does not exist yet. **The fallback only works if
these are guaranteed.**

| Field | Present | Why it is here |
| --- | --- | --- |
| `code` | always | Opaque to Bridge. Looked up, never parsed |
| `message` | always | What renders when the lookup misses |
| `run_id` | always | A process instance exists for every error emitted |
| `fields` | always | May be empty. Never absent, so the fallback has somewhere to read |
| `chain` | always | The flattened cause chain. May be a single entry |
| `job_id` | once a Job exists | Absent on a failure that precedes one |
| `drone_id` | while a Drone runs | A retry is a second id under one Job |
| `step_id` | inside a step | From the WorkflowDef, never generated |

---

## What a person sees

The message names the failure **and the action**. Where it appears is
decided by **blast radius, not severity** — inline, then toast, then
banner, then a full-surface state. Severity picks the edge, not the
placement — and not the colour either: there is one error red, and the
[Design System](design-system.md) contract's error treatment says what
separates a fault from a degraded condition.

### What a person quotes

**The payload is one artifact and one format.** Formatted text, aligned
columns, no fences — it has to survive an issue body, a chat message and a
terminal scrollback, and a fence helps in one of the three. The order is the
order it is read: the guaranteed fields, then `fields`, then `chain` as an
ordered list, then the versions and the instant. The chain is expanded and
never folded, because it is the part that explains the code.

**Absent fields are absent, not empty — with one exception.** A failure that
precedes a Job shows no `job_id` row rather than a blank one. `code` is the
exception and renders `none`: the treatment guarantees a code on every error
and shows it always, so a reader meeting a payload without one cannot tell
whether the failure carried none or whether the paste was cut short, and the
payload is read away from the screen that would have answered that. `none` is a
fact, not a minted code — a code's declaration lives beside the variant that
raises it, so nothing in Bridge is allowed to invent one.

**Three facts are appended that are not on the wire**, because both are the
first thing anyone asks. Both protocol versions, written `bridge protocol 5.2`
— **not an application version**, which Bridge holds nowhere — and when the
payload was **taken**. The instant is labelled, because nothing on the wire
carries when a failure happened and a bare instant appended to an error reads
as the moment it broke.

**Not every failure a person sees is a `WireError`.** Three shapes reach one.
A `WireError` carries everything the table above guarantees. `UnreadableJob` is
a row Fleet refused, sent as a job id and a sentence with no code, no run id
and no chain. An exception caught inside Bridge never crossed a wire at all.
**"Every error carries the payload" is a statement about the artifact, not a
promise that every failure fills it** — and the guarantees in this contract are
the wire's, so they bind exactly one of the three.

**A stack is a chain.** Where the failure is a thrown exception, its frames are
the chain, innermost first. It is the same thing the wire's `chain` is — an
ordered list of causes flattened to strings — and it is the only place a
forty-line value does not destroy a format built on aligned columns.

### What the payload may claim about itself

**Bounded to `fields`, and it says so.** `WireValue` is five primitive variants
and `Secret<T>` implements no `Display` and no `Serialize`, so formatting a
credential into a field does not compile; getting one in needs an explicit
`expose()`, which is deliberate and greppable in one search.

**That reaches nothing else in the payload.** `message` and `chain` are prose,
written by whatever error's `Display` impl raised them, and no type bounds what
an author put there. The sentence Bridge shows states both halves. Stating only
the first would read as a claim about the whole artifact, which is a promise
the mechanism does not make — and none of it says anything about a credential
that was never a `Secret<T>` in the first place, sitting in a repository file
or echoed by a subprocess.

---

## Deliberately outside this

**The v0 lifeboat** is what Bridge still gets when the version check fails
outright — a Fleet too old or too new to speak the current protocol at
all. Rather than nothing, four hand-written routes that do not depend on
version agreement.

| Operation | Route |
| --- | --- |
| List Jobs with status | `GET /v0/jobs` |
| Kill a Job | `POST /v0/jobs/:id/kill` |
| Stop Fleet | `POST /v0/stop` |
| Report Fleet's version | `GET /v0/version` |

Enough to see what is running and stop it. No events, no streaming, and
`curl`-testable by hand. It is also the second reason gRPC was dropped:
there, the lifeboat would have carried a codegen dependency underneath the
one thing whose entire value is having none.

So it keeps its own `{"error": string}` and an HTTP status — no code, no
manifest, no shared type. Its value is being guaranteed to work when
nothing else does, and that holds only while it is small enough never to
need changing. **Reaching into it is the one thing this contract must not
do.**

---

## Open questions

- **[error-occurrence-grouping]** Does Bridge count a repeat occurrence of an
  error on `code` alone, or on `code` plus `step_id`? Raised by the error-states
  drawing on 2026-08-31, which offers an already-filed issue to the second
  occurrence of a code and shows the count. Drawn on `code` alone, and the
  drawing says so is the over-grouping reading: two `judge.undecided` errors on
  different steps of different Jobs are one row under it. Code plus `step_id`
  under-groups in the other direction — the same fault at two steps reads as two
  problems. What decides it is which one a person filing a bug would rather be
  wrong about, and that has not been argued.

Every question the three source proposals raised is answered above. Two only
looked open and were settled elsewhere: Bridge gets no log viewer, and where a
code lives is decided by the collection rule.
