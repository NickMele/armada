// The timer that keeps Pulse's machine reading live while its board is open.
// #1571. The loop is `job-poll.ts`; what is here is what is Pulse's alone.
//
// # The cost
//
// A reading walks a process table and a directory, so this is not free and it
// is why `job-focus.ts` re-took it on events alone until now. One board left
// open for an hour ticks 3600 / 10 = 360 times, all of them local to this
// machine and none of them a forge or a model call. It stops the moment the
// board unmounts and while the window is hidden, so the bound on that cost is
// a person looking at Pulse rather than a Job being open.

import { PULSE_INTERVAL_MS } from "@armada/screens/src/pulse-poll";

import { JobPoll, type JobPollWiring } from "./job-poll";

/** `again` re-takes `/jobs/:job_id/resources`, or a stand-in for a test. */
export type ResourcesPollWiring = Omit<JobPollWiring, "every">;

/**
 * One Job's machine reading, re-taken for as long as Pulse is drawing it.
 *
 * **Not for as long as the Job is open.** A Job is open behind every tab and
 * the reading is drawn by one of them; polling on the Job would pay a process
 * table per tick for a board nobody has on screen.
 */
export class ResourcesPoll extends JobPoll {
  constructor(wiring: ResourcesPollWiring) {
    super({ ...wiring, every: PULSE_INTERVAL_MS });
  }
}
