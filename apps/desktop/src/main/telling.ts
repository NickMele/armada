// Telling somebody a job has started waiting on them, when they are not looking
// at Bridge.
//
// The Board's Needs-you tab already puts a blocked job one keystroke away — but
// only while a window is open. This is the other half: a system notification
// when a job *enters* that set, and a dock count for as long as one is in it.
//
// **The rule is not restated here.** `@armada/screens`' `waiting.ts` holds what
// counts as entering and what one notification says; this holds the interval
// entries are collected over, the effects, and nothing else. That split is why
// this file can be tested in node with no Electron in it — the two effects
// arrive as functions, like every other seam in `src/main`.
//
// # The quiet window, and why there is one
//
// **Notifications that cry wolf get turned off, permanently, by the person they
// were built for.** Five jobs reaching their review gates in the same second is
// one thing to a person and five to the machine, so an entry starts a short
// collection and everything that lands inside it is told once.
//
// The window is deliberately short. It is not a digest and not a throttle: it
// is the width of "at the same time", and a job entering a minute later is a
// second thing that happened and gets its own telling.
//
// # Permission is macOS's, and Armada does not build a second one
//
// The first notification is what makes macOS ask, once, and the answer is the
// person's and is remembered by the system. **A refusal costs nothing here**:
// `show()` becomes a no-op, every other part of Bridge is untouched, and
// nothing asks again on the next launch because nothing here does the asking.
//
// Electron cannot read the authorization state back, so Bridge cannot tell a
// grant from a refusal and does not claim to. What it can do is not depend on
// it: the dock count needs no permission at all, so a person who said no still
// has the standing signal, and only the interruption is lost.

import { entering, telling, waitingIn } from "@armada/screens/src/waiting";
import type { Telling } from "@armada/screens/src/waiting";
import type { JobSummary } from "@armada/protocol";

/**
 * How long entries are collected before one notification goes out.
 *
 * Two seconds: long enough to hold a dispatch of several jobs whose steps
 * finish together, short enough that a single job still arrives while the thing
 * that caused it is what a person was last doing.
 */
export const QUIET_MS = 2000;

/** What this needs the shell to be able to do. Neither is a state. */
export type Effects = {
  /** Post one notification, and land on `to` when it is pressed. */
  show: (told: Telling) => void;
  /**
   * Say how many jobs are waiting, or none.
   *
   * **The standing count, not the new ones.** It answers "is anything waiting
   * on me" for as long as something is, which is a different question from the
   * one a notification answers, and the same number the Board's own sentence
   * draws.
   */
  count: (waiting: number) => void;
  now: () => number;
};

/**
 * What Bridge is holding about who is waiting.
 *
 * **One reading at a time and no history.** The set is every job currently
 * waiting on a person; a reading that arrives is diffed against it and replaces
 * it. Nothing accumulates, which is what makes a resync after an hour offline
 * behave the same as one after a second.
 */
export class Attention {
  private readonly effects: Effects;
  /** The ids waiting on a person, as of the last reading. */
  private waiting: Set<string> = new Set();
  /**
   * Whether a reading has ever landed.
   *
   * **The whole of "already waiting is not news".** Bridge opens holding
   * nothing, so the first reading would otherwise read as every job on the
   * board arriving at once — one notification per job that had been sitting
   * there since Friday.
   */
  private seeded = false;
  /** The last count told to the dock, so an unchanged one is not told again. */
  private counted = -1;
  /** Entries collected inside the current quiet window, by id. */
  private collecting = new Map<string, JobSummary>();
  private window: ReturnType<typeof setTimeout> | null = null;

  constructor(effects: Effects) {
    this.effects = effects;
  }

  /**
   * A fresh reading of every job Bridge holds, and when the list was current.
   *
   * Called on every publish rather than on chosen events: what is diffed is the
   * set, so a reading that changed nothing about who is waiting produces
   * nothing, and there is no event kind to keep in step with the rule.
   *
   * **`readAt` is `null` until Fleet has answered once, and that is refused
   * rather than treated as an empty board.** Bridge publishes state before it
   * is connected to anything; seeding off one of those publishes would make the
   * first real reading look like every waiting job arriving at once, which is
   * the failure the seeding exists to prevent, arriving through the back door.
   */
  read(jobs: readonly JobSummary[], readAt: number | null): void {
    if (readAt === null) return;
    const entered = this.seeded ? entering(this.waiting, jobs) : [];
    this.waiting = waitingIn(jobs);
    this.seeded = true;
    if (this.waiting.size !== this.counted) {
      this.counted = this.waiting.size;
      this.effects.count(this.counted);
    }
    if (entered.length === 0) return;
    for (const job of entered) this.collecting.set(job.id, job);
    this.open();
  }

  /** Drop what is collected and never told. Called when the connection stops. */
  close(): void {
    if (this.window !== null) clearTimeout(this.window);
    this.window = null;
    this.collecting.clear();
  }

  /**
   * Start the quiet window, or leave a running one alone.
   *
   * **It is not extended by a later entry.** Restarting it on every arrival
   * would let a steady trickle of jobs hold the notification back for as long
   * as the trickle lasted, which is the one failure mode worse than too many
   * notifications: none at all, exactly while a lot is happening.
   */
  private open(): void {
    if (this.window !== null) return;
    this.window = setTimeout(() => {
      this.window = null;
      const batch = [...this.collecting.values()];
      this.collecting.clear();
      const told = telling(batch, this.effects.now());
      if (told !== null) this.effects.show(told);
    }, QUIET_MS);
  }
}
