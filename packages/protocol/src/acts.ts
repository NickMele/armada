// The bodies bridge composes and sends — three carrying a person's own words,
// and one carrying a number.
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
// # What the three word-carrying ones have in common makes them three and not one
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
 * The body of `restart_step`. `crates/ipc/src/job.rs`. **The whole body is
 * optional** — a plain restart sends none, which is what every restart sent
 * before this route could read one, so absence has one spelling and there is
 * no `null` inside the type to make a second.
 *
 * **The words reach no session** — a restart exists once the drone is gone.
 * They wait on the job and open the brief of the drone the restart asks for,
 * where a `ChangesRequested` note goes.
 *
 * **The one of the three bridge does not refuse before the press.** A blank
 * field here is a restart with nothing said rather than one that cannot
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

/**
 * The body of `raise_cost_cap`. `crates/ipc/src/raising.rs`.
 *
 * **The one body here that is not a person's own words.** The other three
 * carry a sentence somebody typed; this carries a figure that moves one
 * number on one job.
 *
 * **It raises and never lowers.** Fleet refuses a value at or under the cap
 * in force with a 422 — a call answering 200 while the job is still stopped
 * for money is exactly what the route was built against — and bridge refuses
 * it before the press, matching that.
 *
 * Micros, not dollars: `JobSpend` reads in millionths of a dollar, so the
 * figure a person is shown and the figure that is sent are the same integer.
 */
export type CapRaise = {
  cost_cap_micros: number;
  raised_by: RaisedBy;
};

/**
 * The body of `raise_turn_cap`. `crates/ipc/src/raising.rs`.
 *
 * **Its own type beside `CapRaise`, because the two ceilings are two.** A
 * single body carrying either would mean two things and no outcome could be
 * attributed to one of them — the same argument that keeps the caps two rows in
 * `crates/config/settings.toml` and two routes on the wire.
 *
 * **It raises and never lowers**, on `CapRaise`'s terms and for its reason.
 *
 * A turn count, and no unit conversion anywhere on this act: `JobSpend.turns`
 * and `JobSpend.turn_cap` are the same plain integers, so the figure a person
 * is shown and the figure that is sent are the same number.
 */
export type TurnRaise = {
  turn_cap: number;
  raised_by: RaisedBy;
};

/**
 * Which surface a raise came through, and therefore what it may ask for.
 *
 * **Provenance, never a credential.** Nothing on this seam authenticates
 * anybody, so the field says which surface composed the request and is filled
 * in by that surface — bridge sends `person` because somebody pressed a
 * control, and Helm's tool adapter will send `helm` because it is Fleet's own
 * code wrapping a model's request. A person is unbounded; Helm is not, because
 * an agent that can lift its own budget has no budget.
 *
 * **Bridge only ever sends `person`.** The other value is here because the type
 * is the wire's and the wire carries both — not because anything in this app
 * chooses between them.
 */
export type RaisedBy = "person" | "helm";
