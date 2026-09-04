// What a person may tell a Drone that is still working, and the words for it.
//
// # Why this is not in `recovery.ts`
//
// That file reads `stuck.recourse`, which is Fleet's answer to *this job
// stopped, and here is what moves it*. A job that has not stopped has no stop
// to describe, and `stuck` is absent on one by construction — `Stuck::asked_of`
// admits `escalated` and the three terminals and nothing else, because a
// classification on a running job would be Fleet naming acts against a job
// nothing is wrong with. Widening it to mean "the acts available" is the one
// thing #383 refuses, and this file is the refusal kept.
//
// **A second reading and never a second source.** Nothing here reads `stuck`,
// and nothing in `recovery.ts` reads a healthy drone. A job is stopped or it is
// working, and the two files partition on exactly that.
//
// # What says which acts a working job offers, since Fleet does not
//
// **Presence on the row, which is already what decides every act that is not
// recourse.** `Acts.tsx` draws both kills and the reclaim from the summary and
// says why: recourse is how a stopped job goes forward, and an act that carries
// nothing forward is not one. A redirect into a healthy drone carries nothing
// forward either — the job is already going — so it belongs beside the derived
// acts rather than inside a record of a stop.
//
// `keys.ts` had decided this for this same act one surface over. The Board's
// `d` verb keys off `assigned_drone`, "presence on the row", because a list row
// may not make a second read per row; and what it says about the consequence is
// #383 itself — *a row can offer Redirect where the detail then does not*.
// Since #145 the detail is the wrong one of the two, and it stops being wrong
// by reading the fact the row reads.
//
// **It needs no detail.** The act is on the summary, so it is offered while
// `GET /jobs/:job_id` is still in flight — the opposite of every act on a job
// that stopped, and the same difference stated twice: one of them is a reading
// only Fleet can make, and this one is a pointer.
//
// # The rule this does not break
//
// `docs/journeys/monitor-active-work.md` warns off exactly this pointer, and
// what it warns off is a surface that keys on it **to choose between Redirect
// and Restart Step** — the act is decided by where the job stands, and a drone's
// absence stopped being evidence that anything went wrong once a drone became a
// step's. Nothing is chosen here. A restart is legal only on an escalated job,
// which is `recovery.ts`'s half, so on a working job there is no second act for
// this bit to pick wrongly between: it decides whether the one legal act has
// somewhere to land, which is the question `redirect_drone` itself answers 409
// to.
//
// # What that costs, said rather than hidden
//
// `assigned_drone` is the record's pointer, and a redirect needs the pipe —
// `crates/fleet/src/stuck.rs` reads the slot instead, and says why. The two
// diverge across a Fleet restart until the boot read repairs the record, and in
// that window a redirect offered here answers 409. It is the bargain
// `kill_drone` has always taken on the identical fact, on an act that destroys
// nothing.
//
// # The half that is not here
//
// `docs/concepts/drone.md` gives a redirect at a step boundary a second path:
// the note waits on the job and opens the next drone's brief. Nothing on the
// wire serves that on a running job — `request_changes` writes
// `redirect_waiting` and is refused anywhere but `awaiting_review`, and
// `redirect_drone` answers 409 where there is no session. So a working job with
// no drone on it offers nothing at all here, rather than a note with nowhere to
// go. A person should not have to know which of the two they are getting, and
// that is true; half of it is Fleet's to serve before a surface can hide the
// seam.

import type { JobDetail as JobWhole, JobSummary, RedirectInFlight } from "@armada/protocol";
import type { JobAct } from "./Acts";
import { clock } from "./duration";

/**
 * What a person may say to a drone that is working, and what they read about
 * it.
 *
 * **One act, and the shape holds more than one on purpose.**
 * `docs/concepts/drone.md` puts Kill in the same sentence as Redirect, and Kill
 * is already drawn — from this same pointer, and in the job header where it
 * ends something rather than steers it. It named Pause there too until that was
 * retired on 2026-09-03, for the reason this comment was already giving: no
 * route, no operation implementation, no binding in `actions.toml`. The shape
 * stays open for the act after next, and a control invented here would be this
 * surface minting one.
 */
export type Steering = {
  /**
   * The one act this reading offers. Absent is a job with no drone on it,
   * which is a queued job, a job between steps and a job waiting on a slot.
   */
  act?: "redirect";
  /**
   * One sentence per act on offer, keyed by the act, for that act's own
   * tooltip — the same shape `Recourse.says` carries, so the control draws its
   * help the one way whichever reading offered it.
   */
  says: Partial<Record<JobAct, string>>;
  /**
   * The redirect that is out and unanswered. **On a healthy drone it is the
   * only thing that says anything happened**, because the job is `running`
   * before the send and `running` after the answer — no status moves, and
   * `crates/fleet/src/resume.rs` says so where it waits on the turn.
   */
  sent?: string;
};

/**
 * What can be said to this job's drone now.
 *
 * The detail is a second argument for one field of the answer and not for the
 * act: `redirecting` is on `GET /jobs/:job_id` and on no board row, and it is
 * absent both while that read is in flight and when nothing is outstanding.
 * The act itself is decided before the read lands.
 */
export function steeringOf(job: JobSummary, whole: JobWhole | null): Steering {
  if (job.assigned_drone === undefined) return { says: {} };
  const sent = whole?.redirecting;
  return {
    act: "redirect",
    says: { redirect: STEER },
    ...(sent === undefined ? {} : { sent: waiting(sent) }),
  };
}

/**
 * What a redirect into a working drone is, in the shape the other acts are
 * stated in: what it reaches, what it keeps, and what it does not do.
 *
 * **Never a step restart.** `docs/concepts/drone.md` makes that the rule rather
 * than a preference — the work already done is kept and the record says a
 * person intervened — so the sentence says what survives before it says
 * anything else.
 */
const STEER =
  "The drone is working and its session is open, so an instruction reaches it as a new turn at " +
  "the step above. Nothing is spawned, nothing already done is thrown away, and the step is not " +
  "started again.";

/**
 * The redirect that is out, on a job that never stopped.
 *
 * **It is not a status and it does not claim delivery**, which is
 * `recovery.ts`'s reading of the same field; what differs is the half after the
 * comma. There the job is held at `escalated` and the drone's turn is what
 * releases it. Here nothing is held, so the turn moves nothing and the only
 * thing a person can be told is that fleet wrote to the pipe.
 *
 * **The time is a clock, not an age.** Nothing ticks on the wire and nothing
 * counts up here.
 */
function waiting(sent: RedirectInFlight): string {
  return (
    `Sent, waiting for the drone. The instruction went into its session at ${clock(sent.sent_at)}, ` +
    "and nothing here moves when it lands — this job was never held, and it goes on working " +
    "either way. Redirecting again replaces what is outstanding."
  );
}
