// The timer behind a destination that has to stay live while nobody touches
// the Job it is reading.
//
// **One shape, two uses.** `remarks-poll.ts` (`#667`) was the first, and when
// Pulse needed the same thing (`#1571`) the choice was a second copy of this
// loop or this file. A copy would have been two places to fix the rule below
// about a tick that is skipped rather than queued, so the mechanism moved here
// and each caller keeps what is actually its own: how often it asks, and what
// that costs.
//
// Nothing here knows a route, a socket or a port. `again` and `port` are both
// handed in, and read fresh on every tick — the connection this is asking over
// is built long after the poll is.

export type JobPollWiring = {
  /** How often an on-screen destination re-asks. Its caller's number, not this file's. */
  every: number;
  /** The port to ask over, read fresh on every tick. Never held. */
  port: () => number | null;
  /** The re-read itself, for whichever Job is watched. */
  again: (port: number, jobId: string) => Promise<void>;
};

/**
 * One Job's read, taken again for as long as the surface drawing it is open.
 *
 * **One Job at a time.** A person has one of these open; `watch` replaces
 * whichever Job was held rather than adding a second timer behind it, which
 * would go on paying for a surface nobody is reading.
 */
export class JobPoll {
  private readonly wiring: JobPollWiring;
  private jobId: string | null = null;
  private windowShown = true;
  private timer: ReturnType<typeof setInterval> | null = null;
  /** Never more than one read in flight — a tick is skipped, not queued. */
  private inFlight = false;

  constructor(wiring: JobPollWiring) {
    this.wiring = wiring;
  }

  /** The Job whose surface is on screen, or `null` once it leaves. */
  watch(jobId: string | null): void {
    this.jobId = jobId;
    this.settle();
  }

  /** Whether Bridge's window is on screen. A minimized window counts as hidden. */
  shown(visible: boolean): void {
    this.windowShown = visible;
    this.settle();
  }

  /** Starts or stops the interval to match what is now watched and shown. */
  private settle(): void {
    const wants = this.jobId !== null && this.windowShown;
    if (wants && this.timer === null) {
      this.timer = setInterval(() => void this.tick(), this.wiring.every);
    } else if (!wants && this.timer !== null) {
      clearInterval(this.timer);
      this.timer = null;
    }
  }

  /**
   * One tick. **Skipped outright while the last one is still out**, rather
   * than queued behind it — a slow answer catching up on every tick that
   * piled up while it ran is the second read in flight this class exists to
   * refuse.
   */
  private async tick(): Promise<void> {
    if (this.inFlight) return;
    const jobId = this.jobId;
    const port = this.wiring.port();
    if (jobId === null || port === null) return;
    this.inFlight = true;
    try {
      await this.wiring.again(port, jobId);
    } finally {
      this.inFlight = false;
    }
  }
}
