// The three bodies a person's own words cross in.
//
// # Cut out of `protocol.ts`, and the cut is not the line count
//
// That file reached the 900 lines the gate refuses when `RestartRequested`
// landed, and a split made to get under a number moves the metric rather than
// the coupling. This one is a subject that was already separate: every other
// type in `protocol.ts` is something fleet answers *with*, and these three are
// the only ones bridge composes and sends. `ChangesRequested` had already left
// for `work.ts` on the same reasoning; these three stayed by history.
//
// # What they have in common is what makes them three types and not one
//
// All three are structurally one string, and each says who reads it. A redirect
// steers a drone that is there. A restart's note reaches a drone that does not
// exist yet. An override's reason reaches no drone at all — it is written for
// the record. One shared body would be one route meaning whichever the caller
// had in mind, which is the argument `crates/ipc` makes for keeping them apart
// on its own side.
//
// **Blank is refused server-side on all three**, with a 422 rather than a 400:
// a decoded request is well-formed, and a string with nothing in it is a value
// that cannot work. Bridge refuses two of them before the press as well. The
// third is the exception and says why.

/**
 * The body of `redirect_drone`. `crates/ipc/src/job.rs`. The one string that
 * reaches a drone without fleet assembling it — blank is refused server-side.
 */
export type Redirection = {
  instruction: string;
};

/**
 * The body of `restart_step`, and **the whole body is optional**.
 * `crates/ipc/src/job.rs`.
 *
 * A plain restart sends none, which is the request every restart sent before
 * this route could read one — so absence has one spelling, and there is no
 * `null` inside the type to make a second.
 *
 * **The words reach no session**, because a restart is the act that exists once
 * the drone is gone. They wait on the job and open the brief of the drone the
 * restart asks for, which is where a `ChangesRequested` note goes.
 *
 * **The one of the three bridge does not refuse before the press.** A blank
 * field here is a restart with nothing said rather than a restart that cannot
 * happen, so it is dropped and the act goes through.
 */
export type RestartRequested = {
  note: string;
};

/**
 * The body of `override_verdict`. `crates/ipc/src/work.rs`.
 *
 * **Its own type though it is structurally the same string as `Redirection`**,
 * for that type's own reason turned around: a redirect steers a drone, and this
 * one goes nowhere near one. It is written for the record and for whoever later
 * asks how often the judge was wrong. Blank is refused server-side with a 422,
 * and refused here before the press for the same reason.
 */
export type Overruled = {
  reason: string;
};
