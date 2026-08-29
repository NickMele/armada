//! What this crate proves about itself so far.
//!
//! The OS lifecycle is three subjects that are one subject: a Fleet that
//! outlives the app is only useful if something can find it, and a runtime file
//! is only useful if its reader can tell a live Fleet from a number that used
//! to be one.
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
//! recorded and the Drone may declare again — plus the case that asserts
//! nothing new, a step with no scope behaving exactly as it always did.
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
//! `settling` is the sixteenth, and every case in it is one where nothing
//! advances: the gate asked and refusing. What it proves is that a refusal
//! keeps the submission where it can still be ruled on, says which guard
//! refused, and escalates rather than sitting once nothing can reach it.
//!
//! `overruling` is the seventeenth, and its proportion is the claim: one case
//! where a refused step advances because a person disagreed with the Judge, and
//! four where nothing moves. What keeps an appeal from being an
//! approve-anything is the four.
//!
//! `redirect` is the eighteenth: steering a Drone that is still there. Its
//! claim is the pair a single predicate used to conflate — a Job escalated over
//! a **live** Drone with no step stopped takes a redirect and nothing else
//! does, and it comes back to `running` on the Drone's own next turn rather
//! than on the sending. Most of its cases are ones where the Job stays
//! escalated, which is the proportion the act is about.
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

mod attachments;
mod briefing;
mod checks;
mod converging;
mod daemon;
mod delivery;
mod detach;
mod detail;
mod drone;
mod dry_run;
mod evidence;
mod footprint;
mod frozen;
mod gaming;
mod gate;
mod history;
mod host;
mod http;
mod judging;
mod landing;
mod overruling;
mod planning;
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
mod serving;
mod session;
mod settling;
mod silence;
mod stuck;
mod tmp;
mod transcript;
