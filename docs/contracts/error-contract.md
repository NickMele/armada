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
by collection, not by authorship: a `verify-foundations` rule walks both
halves of the repository and fails on a code declared twice, naming every
site that declares it.

A central registry puts every code far from the failure it names and turns
adding one into a merge conflict. Scanning gets the closure without the
distance — the same shape the vendor-literal rule already uses.

**What a declaration looks like is not specified here, it is observed.** The
scan reads the form each half already uses, so neither had to change:

| Half | Declared as | Found by |
| --- | --- | --- |
| Rust | `const NO_SUCH_JOB: &str = "fleet.no_such_job";` | the literal's shape |
| Bridge | `const FLEET_UNREACHABLE: BridgeCode = "bridge.fleet.unreachable";` | the type annotation |

Bridge's half is exact, because `BridgeCode` is a type. Rust's is a shape —
two or more lowercase segments — because nothing types a code there, and a
dotted lowercase string is also what a file name looks like. Four of those
live in `crates/` and are excluded by their extension. **That exclusion is a
guess, and it is the loud kind**: a wrong include arrives as a duplicate
naming a file name, not as silence.

### A code's meaning

**The meaning is the doc comment beside the declaration, and no manifest is
checked in.** This contract used to say the meaning was lifted into a
generated, tracked file so that changing what a code means changed a diff.
It already does: the doc comment is in a tracked file, one line above the
declaration, and a generated copy of it moves in exactly the same commit.
The copy would add no detection and one more way to be stale.

`cargo xtask verify-error-codes` prints the collection instead — every code,
its site, and its one line — computed on demand. **The listing is the
manifest, and it cannot go out of date because it is never written down.**

Two alternatives rejected. A stated convention against repurposing, honest
about being unenforceable, is what this replaces and is weaker than a
comment a reviewer sees in the diff. Checking in the listing was rejected on
the argument above and on what this repository has already found out about
generated files nobody reads — see #468.

**The residual risk is named rather than closed.** A meaning can still shift
under a stable name if somebody edits the doc comment and the reviewer waves
it through; nothing here makes that impossible. What the manifest would have
added is a second place for the same reviewer to miss it.

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

**A rule in `verify-foundations`, and `cargo xtask verify-error-codes` runs
that same rule alone.** Two doors to one check, the way `verify-tokens` is
also a `verify-foundations` rule.

This page used to say the check stood alone, and the argument it stood on
was about the *protocol* check: folding into that one would couple a
contributor adding a Rust code to the whole TypeScript codegen chain, and
the failure would read *protocol is stale* when the fault is two variants
claiming one code. Both halves of that argument survive and neither points
at `verify-foundations`, which builds nothing, needs no toolchain and prints
each rule under its own name — so the failure still reads *one code, one
failure* and not something else.

What changed the answer is that a command outside the gate is a command
nobody runs. **A check that names its own failure is worth more than a
shorter list of checks; a check nothing invokes is worth nothing at all.**

### Version skew

Adding a code is **minor**. Removing one is **also minor** — a deliberate
departure from the general rule that removing anything is major.

Bridge never matches codes exhaustively; it looks one up or falls back. A
code it has never heard of and a code that has been withdrawn are the same
event to it, and both render.

---

## What the wire always carries

Bridge falls back to these when it meets an unknown code, and every code it
meets is unknown: it looks one up or falls back, and never matches
exhaustively. **The fallback only works if these are guaranteed.**

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
payload is read away from the screen that would have answered that.

**Bridge mints a code for each of its own faults.** Only one of the failures
Bridge draws crosses the wire that guarantees one, and the code is what
separates an error from a failed Job. See #228.

**A Bridge code is declared beside the builder that raises it**, in
`packages/shell/src/failures.ts`, and is namespaced `bridge.`. The prefix keeps
the Rust set and the Bridge set disjoint without a collector spanning both —
which is why the collection above may check each half against itself and
decide nothing about the other. A Rust code that took the prefix fails, since
it is the one thing that would make that reading wrong.

**`none` is a fact, not a minted code.** No failure Bridge draws renders it; it
is what a payload assembled outside those builders shows.

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

### What a person files

**Copying stays on the machine. Filing leaves it**, so they are two acts and
only the second one has a review. `Copy debug info` puts the payload on the
clipboard in one press. **File an issue** opens a dialog naming every item that
would go, with its text on screen and a control to take it out. **Send is never
one press from an error.**

**It appears where the payload is legible in full** — a full-surface error and
an expanded view — and never on a toast or an inline error. A review needs the
artifact on screen, and a toast is gone before it would be read.

**Armada makes no scrub claim.** It states that nothing is removed on the way
out and puts the exact text of every item on the row it belongs to. A promise it
cannot keep is worse than the work of reading — and the mechanism that bounds
`fields` reaches nothing else, which is why the error's own record carries the
same sentence pair the expanded view shows rather than being waved through as
structured and safe.

**The record cannot be taken out, and that is not a safety claim.** An issue
with the record removed is a sentence somebody typed, which is the thing this
replaces. Required is about the report being answerable, not about the artifact
being bounded — and an item a person cannot remove is the item they had most
better have read.

**Nothing is sent, and the dialog says so.** Fleet holds no credential for an
issue tracker and nothing on the wire names the repository's remote, so filing
produces a body and puts it on the clipboard. Two things follow and are not
built: a **Reported** strip carrying the issue link and the time, and offering
an already-filed issue to the second occurrence of a code. Both need an issue
number, and none ever reaches Bridge. **The control therefore stays after a
filing**, and nothing on the error says one happened — a strip saying "reported"
on the evidence that somebody pressed copy would be Armada claiming to know
something it does not. **The body carries what was attached and what was not, by
name** — with no strip, it is the only place that answer lives.

**The record is the only item offered today.** Doctor is not built; a Judge
response and a diff belong to a Job read whole, which no failure surface holds;
and whether an observed transcript may leave the machine is
`[observe-transcript-sharing]` on [Observe](../concepts/observe.md), which is
open and names attaching one to a bug report as removing today's bound. Only the
transcript's absence is said on screen, because it is the only one somebody
looks for and finds missing.

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

- **[filing-transport]** How does a filed issue actually reach a tracker, and
  whose act is the send? Filing is specified above and built, and it ends at
  the clipboard: Fleet holds no credential for an issue tracker and nothing on
  the wire names the repository's remote, so the dialog says outright that
  Armada opens nothing and the last step is the person's. Three things decide
  it, and none has been argued. **Where a credential lives** — Armada brokers
  credentials to Drones and `Secret<T>` is the type that carries one, but a
  tracker credential is the app's rather than a Job's, and no config surface
  claims it. **What names the remote** — a Manifest knows its repository and
  the wire does not carry it, so either the operation grows a field or Bridge
  reads it from somewhere that is not the wire. **Whether Armada sends at all**
  — `crates/ipc/operations.toml` already records the strongest `No` in the file
  against an agent filing on the owner's behalf, on the grounds that the record
  is evidence and his reason is the finding; whether that reasoning extends to
  the owner pressing Send himself is a different question and is not answered
  there. Two built-and-refused things wait on this and are named above: the
  **Reported** strip, and offering an already-filed issue to a second
  occurrence. Both need an issue number, which only a real send produces —
  which is also why `[error-occurrence-grouping]` below has nothing waiting on
  it today.

- **[error-occurrence-grouping]** Does Bridge count a repeat occurrence of an
  error on `code` alone, or on `code` plus `step_id`? Raised by the error-states
  drawing on 2026-08-31, which offers an already-filed issue to the second
  occurrence of a code and shows the count. Drawn on `code` alone, and the
  drawing says so is the over-grouping reading: two `judge.undecided` errors on
  different steps of different Jobs are one row under it. Code plus `step_id`
  under-groups in the other direction — the same fault at two steps reads as two
  problems. What decides it is which one a person filing a bug would rather be
  wrong about, and that has not been argued. **Nothing waits on it today**: the
  offer it counts for needs an issue number to point at, Bridge never receives
  one, and no occurrence counting is built.

Every question the three source proposals raised is answered above. Two only
looked open and were settled elsewhere: Bridge gets no log viewer, and where a
code lives is decided by the collection rule.
