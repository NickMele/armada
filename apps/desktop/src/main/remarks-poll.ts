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

/** How often an on-screen panel re-asks. The cost above is measured at this. */
export const INTERVAL_MS = 20_000;

export type RemarksPollWiring = {
  /** The port to ask over, read fresh on every tick. Never held. */
  port: () => number | null;
  /** `ReviewMaterial.remarksChanged`, or a stand-in for a test. */
  again: (port: number, jobId: string) => Promise<void>;
};

/**
 * One Job's comments, re-asked for as long as its panel is on screen.
 *
 * **One Job at a time.** A person reviewing has one panel open; `watch`
 * replaces whichever Job was held rather than adding a second timer behind
 * it, which would double the cost above for a panel nobody is reading.
 */
export class RemarksPoll {
  private readonly wiring: RemarksPollWiring;
  private jobId: string | null = null;
  private windowShown = true;
  private timer: ReturnType<typeof setInterval> | null = null;
  /** Never more than one read of a PR in flight — a tick is skipped, not queued. */
  private inFlight = false;

  constructor(wiring: RemarksPollWiring) {
    this.wiring = wiring;
  }

  /** The Job whose comments are on screen, or `null` once they leave it. */
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
      this.timer = setInterval(() => void this.tick(), INTERVAL_MS);
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
