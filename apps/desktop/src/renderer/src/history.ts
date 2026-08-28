// One Job's transition history, from `GET /jobs/:job_id/events`.
//
// **Rendered, never replayed.** `crates/store/src/fold.rs` owns the machine and
// is the only thing that may put a recorded move back through
// `Job::transition`. Nothing here folds, infers a status the log did not carry,
// or repairs a sequence it does not like — Fleet loads the Job before it reads
// the log, so a history that arrives is one the machine already admitted.
//
// **`seq` orders it, never `at`.** The instant is injected rather than read
// from a clock, so two moves inside one millisecond carry the same one. Sorting
// on the instant would put a step move and the status move it caused in
// whichever order the transport happened to deliver them, and which caused
// which is the whole question this surface is opened to answer.
//
// **The log's own spellings render.** A status, a step state and a Drone
// presence are all drawn as the row stores them, because this is the surface a
// person reads in order to go and look at the row. The reason is the one thing
// turned into words, through the registry that owns the wording — the same
// substitution the rail already makes for a verdict's trigger.

import type { TransitionMove } from "@armada/components";

import { ESCALATION_REASON, QUEUED_REASON } from "../../shared/generated/vocabulary";
import type { Recorded, StepMoved } from "../../shared/history";
import type { Reason } from "../../shared/protocol";
import { clock } from "./duration";

/**
 * The rows, oldest first.
 *
 * **Sorted on `seq`, though Fleet already sends them in it.** The order is the
 * whole content of a history, and a client that took delivery order for it
 * would be trusting the transport with the one thing it is reading.
 */
export function movesOf(moves: Recorded[]): TransitionMove[] {
  return [...moves].sort((a, b) => a.seq - b.seq).map(moveOf);
}

function moveOf(recorded: Recorded): TransitionMove {
  const common = { seq: recorded.seq, at: clock(recorded.at), actor: recorded.actor };
  const moved = recorded.moved;
  if (moved.kind === "status") {
    // `Recorded.status` is where it left, and `to` is where it arrived. There
    // is no `from` on the wire for that reason, and none is composed.
    return {
      ...common,
      kind: moved.kind,
      moved: `${recorded.status} → ${moved.to}`,
      why: whyMoved(moved.to, moved.reason),
    };
  }
  if (moved.kind === "step") {
    return {
      ...common,
      kind: moved.kind,
      subject: moved.step_id,
      moved: `${moved.from} → ${moved.to}`,
      why: whyStepMoved(moved),
    };
  }
  // A Drone arrived or left. Presence, not a state pair — so there is one word
  // and no arrow, and inventing a `from` for it would be drawing a machine
  // `assigned_drone` does not have.
  return { ...common, kind: moved.kind, subject: moved.drone_id, moved: moved.presence };
}

/** The two step states the pair below is read off. */
const STOPPED = "stopped";
const ADVANCED = "advanced";

/**
 * The trigger a step move carried, in the registry's words. Absent on every
 * move that carries none, which is most of them.
 *
 * **`stopped → advanced` is an override and never a stop.** Both moves that
 * carry a trigger carry the same one — the step stopped on it, and then a
 * person overruled it — so the verb alone reads as though the Judge refusing is
 * why the step advanced, which is the opposite of what happened.
 * `crates/ipc/src/event.rs` names the pair as what tells the two apart, and a
 * history row is the one surface that has to read it: the detail is served an
 * `overridden` field so no rail works the pair out, and no such field exists on
 * a recorded move.
 */
function whyStepMoved(moved: StepMoved): string | undefined {
  if (moved.why === undefined) return undefined;
  const named = ESCALATION_REASON[moved.why]?.verb ?? moved.why;
  // The word is the actor's act, and the trigger it lifted is kept beside it —
  // what was overruled is the whole of what makes an override readable.
  return moved.from === STOPPED && moved.to === ADVANCED ? `overruled · ${named}` : named;
}

/**
 * Why a status move happened, where the transition stored a reason.
 *
 * **The vocabulary is the destination's.** A move to `escalated` carries an
 * escalation trigger and a move to `queued` carries a readiness reason, so the
 * lookup is keyed on where it arrived — the same rule `readingOf` states for a
 * Job's own badge, applied to a row that has both ends.
 *
 * Absent on the eight destinations that store none, which is most moves, and
 * absent rather than an empty string: a row that said nothing about why would
 * be indistinguishable from one whose reason was lost.
 */
function whyMoved(to: string, reason: Reason | undefined): string | undefined {
  if (reason === undefined) return undefined;
  const named = reason.named;
  const held = named === undefined ? undefined : registry(to, named);
  const owed = reason.criteria_owed ?? [];
  const parts = [
    ...(held === undefined ? [] : [held]),
    ...(owed.length === 0 ? [] : [`owes ${owed.join(", ")}`]),
  ];
  return parts.length === 0 ? undefined : parts.join(" · ");
}

/** The reason in words, or the spelling where the registry carries no verb. */
function registry(to: string, named: string): string {
  if (to === "queued") return QUEUED_REASON[named]?.verb ?? named;
  if (to === "escalated") return ESCALATION_REASON[named]?.verb ?? named;
  return named;
}

/**
 * Why a history has no rows, which is never the same sentence twice.
 *
 * **Empty is a real answer and is not a failure.** A Job created and not yet
 * moved has no events at all — creation is not a transition, it has no `from`,
 * and no row describes it.
 */
export const NOTHING_RECORDED =
  "This job has not moved yet. Creation is not a transition, so no row describes it.";

/** What the list is, and what it is not. The turns answer the other question. */
export const WHAT_THIS_IS =
  "What Armada did, in the order the log recorded it. What the drone said is in its turns.";
