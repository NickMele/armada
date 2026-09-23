// The 20 s timer that keeps one Job's comments current while its review panel
// is on screen. #667.
//
// Its own module rather than a piece of `connection.ts`, which sits near this
// workspace's 900-line file cap and has no line to spare for a timer.
//
// **It shares #666's re-fetch rather than opening a second one.** The one call
// this makes is `ReviewMaterial.remarksChanged` — the same route
// `job.remarks_changed` already calls off Fleet's sweep — so a comment the
// sweep finds and a comment this timer finds land through the one path that
// keeps a ticked box and a typed note in place. `index.ts` wires the two
// arguments below: the port, read fresh off what main last published, and
// `remarksChanged` itself.
//
// # The cost
//
// `get_remarks` is two `gh` calls, one for the conversation and one for the
// inline comments — see `review.ts`. One panel left open for an hour ticks
// 3600 / 20 = 180 times, so 360 `gh` calls in that hour: about 7% of the
// 5,000 requests an hour a signed-in person's token gets on the forge's REST
// API. `INTERVAL_MS` is what to raise if that cost ever needs to move.

// The loop itself is `job-poll.ts`, shared with Pulse's since `#1571`. What
// stays here is this panel's alone: its interval, and what that interval costs.
import { JobPoll, type JobPollWiring } from "./job-poll";

/** How often an on-screen panel re-asks. The cost above is measured at this. */
export const INTERVAL_MS = 20_000;

/** `again` is `ReviewMaterial.remarksChanged`, or a stand-in for a test. */
export type RemarksPollWiring = Omit<JobPollWiring, "every">;

/** One Job's comments, re-asked for as long as its panel is on screen. */
export class RemarksPoll extends JobPoll {
  constructor(wiring: RemarksPollWiring) {
    super({ ...wiring, every: INTERVAL_MS });
  }
}
