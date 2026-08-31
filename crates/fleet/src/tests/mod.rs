//! What this crate proves about itself so far.
//!
//! The OS lifecycle is three subjects that are one: a Fleet that outlives the
//! app must be findable, and a runtime file must let its reader tell a live
//! Fleet from a number that used to be one.
//!
//! `process` proves the identity primitive, `runtime` proves the file built on
//! it, `detach` proves a Drone is not in Fleet's process group. Nothing here
//! reads a clock, and nothing here is skipped on a platform: `ps` is spelled
//! the same way everywhere Armada runs.
//!
//! `gate` is the fourth and it is a different subject: what Fleet does when
//! Evidence lands. Most of its cases are cases where nothing advances, which is
//! the proportion the milestone is about.
//!
//! `drone` and `session` are the fifth: what a Drone is given, what it is told,
//! and what its death means for the Job. They start a real child — a shell that
//! prints something and reads a line — because the two questions they exist to
//! answer are questions about an operating system rather than about a value.
//! **Nothing here starts an agent.** What a Drone is confined to is a rendering
//! and is asserted in `adapters`, where no process is involved at all.
//!
//! `evidence` is the seventh: a tool call arriving as JSON-RPC over the router
//! that ships, reaching the inbox, and advancing the step it was for.
//!
//! `frozen` is the eighth: what a running Job keeps hold of. Its workflow does
//! not move under it, it knows which Drone is on it, and its Checks leave their
//! output on disk.
//!
//! `landing` is the ninth: a finished Job's work reaches its branch. Most of its
//! cases are ones where no commit is made, which is where the rule is.
//!
//! `scope` is the eleventh: evidence bound to what the step actually touched.
//! Its cases are the two moments the comparison runs at — the gate, where a
//! footprint outside the declaration fails the step, and the turn, where it is
//! recorded and the Drone may declare again — plus a step with no scope
//! behaving exactly as it always did.
//!
//! `judging` is the tenth: the semantic tier, run through Fleet's own process
//! runner against a shell that prints a scripted verdict. Its three cases are
//! the milestone's claim — a veto stops a step whose Check passed, a
//! no-objection lets it advance, and a call that failed does neither.
//!
//! `daemon` and `serving` are the sixth, and they are the first that are about
//! the whole: a Job driven from created to completed against fakes, read back
//! out of a reopened store, and the same five operations answered over the
//! router that ships. `briefing` and `host` are the two seams that arrived with
//! them — what a Drone is told, and the one place a clock is read and an id is
//! invented.
//!
//! `footprint` is the thirteenth: what a Drone has changed on disk, while it is
//! still changing it. Most of its cases assert that nothing was read — an idle
//! Fleet, a Fleet nobody has open, a turn inside the interval — because the
//! risk in a live view is a repository read on a 250ms loop that nobody asked
//! for.
//!
//! `converging` is the twelfth: a Drone working and not getting anywhere. Most
//! of its cases assert an absence — a tripwire that escalates nothing, a look
//! that stops the chain, a Drone that reports when told to — because what the
//! chain is for is refusing to escalate early.
//!
//! `history` is the fourteenth, and it is the one read that does not answer
//! from the fold: a Job's log, in the order it was written, over the router
//! that ships. What it proves is that the sequence survives — both machines and
//! the Drone's arrival among each other, each row standing where the last one
//! left the Job — and that reading it replays nothing.
//!
//! `reviewing` is the fifteenth: the material a person decides on and the
//! three answers they may give. Its cases are the refusals as much as the acts
//! — a review that reaches a Job nobody was shown, and a note with nobody to
//! tell — because those are what keep the three from becoming acts they are
//! not.
//!
//! `settling` is the sixteenth, and every case is the gate asked and refusing:
//! a refusal keeps the submission where it can still be ruled on, says which
//! guard refused, and escalates rather than sitting once nothing can reach it.
//!
//! `overruling` is the seventeenth, and its proportion is the claim: one case
//! where a refused step advances because a person disagreed with the Judge, and
//! four where nothing moves — which is what keeps an appeal from being an
//! approve-anything.
//!
//! `redirect` is the eighteenth: steering a Drone that is still there. Its
//! claim is the pair a single predicate used to conflate — a Job escalated over
//! a **live** Drone with no step stopped takes a redirect and nothing else
//! does, and it comes back to `running` on the Drone's own next turn rather
//! than on the sending.
//! `dry_run` is the nineteenth, and it is `gate` asked from the other side: a
//! Drone finding out where it stands before it spends a step finding out the
//! hard way. One case gives it an answer and five prove an absence — the step
//! does not move, the gate reaches its own verdict on its own run, the
//! convergence clocks do not count the wait, and each of the two bounds refuses
//! a call.
//!
//! `attachments` is the eleventh: a file staged before a Job exists, promoted
//! at creation, refused where the staged path cannot be read, and copied again
//! into the worktree dispatch makes — proving the same path `briefing` writes
//! into a Drone's first turn is one the worktree actually holds.
//!
//! `checking` is the twenty-first, and it is what concurrency was allowed to
//! change: how long a step's Checks take, and nothing else. Three of its four
//! cases assert that something stayed as it was — the order of the report, the
//! row a skip occupies, and the budget a queued Check has not begun spending.
//!
//! `forget` is the twentieth: deleting a terminal Job's whole record. It is
//! not a transition, so most of its cases prove an absence — the row is gone
//! from a reload and from `get_job`, a Job still in flight is refused rather
//! than moved — plus the one positive case, the event that tells a watching
//! client which id to drop.
//!
//! `coupling` is the twenty-third: what an upstream's terminal status does to
//! the Job waiting behind it. `planning` and `queued` already prove the
//! ordering, so every case here is one of the two outcomes that used to leave a
//! Job at `queued` for ever — the failure that never self-clears, and the
//! `superseded` that was meant to be the graceful one — plus the refusal that
//! makes a cycle unstatable rather than merely undetected.
//!
//! `concurrency` is `#50`'s own claim and the twenty-fifth: two approved Jobs
//! worked at the same moment, each in its own worktree, and each Drone's tool
//! call landing on the step that made it. Its last two cases are the ones that
//! matter — the declarations follow the *connection* rather than the admission
//! order, and a caller nothing holds reaches neither Job — because without them
//! every assertion in the file would also pass against the single slot.
//!
//! `peer` is the twenty-fourth, and it is the only thing here that asserts
//! against the kernel's own idea of who holds a socket. Its first case is the
//! one that matters: it opens a second connection from the same process to
//! somewhere that is not Fleet and asserts the pair refuses it, because a
//! lookup keyed on the local port alone would say yes and would be wrong
//! deterministically. See `crate::peer`.
//!
//! `headroom` is `#44`'s and the twenty-sixth: a Job held back because the
//! machine is short, and a running one left alone when it fills.
//!
//! `starting` is the twenty-seventh: a Job that never started, and which of the
//! three triggers upstream of a spawn its badge names. Every case asserts the
//! trigger rather than the `Adrift`, because the error was always right and the
//! one word a person reads before they open anything was not.
//!
//! `boundary` is the twenty-second, and the only thing here that asks the
//! operating system what happened to a Drone. Every other boundary test asserts
//! bookkeeping, and would pass over a `setsid`-detached Drone still running.
//! Two of its three cases are a pair: gone after it is stood down, still there
//! after the slot is merely dropped.
//!
//! `keeping` is the twenty-sixth, and the only one that deletes a worktree
//! deliberately. `delivering` proves the Judge is shown the file a step was
//! asked to write; this one proves that file is still readable after the Job it
//! belongs to has been cleaned, which it was not — `#223`. Its other cases are
//! all absences, because the risk in keeping a copy is keeping the wrong one:
//! nothing is kept of a document too big to have been judged, nothing of a step
//! the mechanical tier stopped, and no copy is ever written over.

mod asked;
mod attachments;
mod attribution;
mod boundary;
mod bounding;
mod briefing;
mod capacity;
mod checking;
mod checks;
mod concurrency;
mod converging;
mod coupling;
mod crossing;
mod daemon;
mod delivering;
mod delivery;
mod detach;
mod detail;
mod drone;
mod dry_run;
mod evidence;
mod footprint;
mod forget;
mod frozen;
mod gaming;
mod gate;
mod headroom;
mod history;
mod host;
mod http;
mod judging;
mod keeping;
mod landing;
mod modelling;
mod overlap;
mod overruling;
mod peer;
mod planning;
mod planted;
mod preparing;
mod process;
mod proposing;
mod queued;
mod redirect;
mod redispatch;
mod regating;
mod reporting;
mod restarting;
mod retrying;
mod reviewing;
mod runtime;
mod scope;
mod sending_back;
mod serving;
mod session;
mod settling;
mod silence;
mod starting;
mod stuck;
mod tmp;
mod tools;
mod transcript;
