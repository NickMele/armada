// The open Job, held whole and kept current, and the two reads beside it that
// a person asks for by name: an examination, and the Job's own transition
// history.
//
// Pulled out of `connection.ts` for the reason every module beside it was:
// nothing here belongs to the connection except the port, the current state,
// and the sockets a reconnection has to bring back together — `screen.ts`
// still owns which regions those are and why they travel as one list.

import type { BridgeState } from "../shared/bridge";
import type { JobDetail, JobExamined, JobHistory, JobResources, Recorded } from "@armada/protocol";
import type { JournalSocket } from "./journal";
import type { ObserveSocket } from "./observe";
import { JobReader } from "./reader";
import { ask } from "./request";
import type { ReviewMaterial } from "./review";
import { takeAgain, type Again } from "./screen";

export type JobFocusWiring = {
  port: () => number | null;
  current: () => BridgeState;
  now: () => number;
  publish: (change: Partial<BridgeState>) => void;
  turns: ObserveSocket;
  notes: JournalSocket;
  review: ReviewMaterial;
  observing: () => string | null;
  reading: () => string | null;
};

export class JobFocus {
  private readonly wiring: JobFocusWiring;
  /**
   * The open Job, read whole and kept current. Here rather than in the renderer
   * because every event naming this Job re-reads it, which is what makes a rail
   * redraw when a step advances.
   */
  private readonly watched: JobReader<{ detail: JobDetail }>;
  /**
   * What the open Job holds on this machine.
   *
   * **Opened with the Job and re-read on every event naming it**, which is the
   * same rule `watched` follows and for the same reason: a figure that stopped
   * moving while a Job ran would be a panel claiming a stall that is not there.
   *
   * **No timer.** Every reading walks a process table and a directory, and a
   * poll would pay for that continuously to answer a question asked rarely —
   * which is the cost the Fleet side already refuses. A Job that has genuinely
   * wedged emits no events and so goes stale, which is why `read_at` is on the
   * wire and why `examineJob` is the press that takes a fresh one.
   */
  private readonly resources: JobReader<{ resources: JobResources }>;
  /**
   * The open Job's transition history, where a surface unfolded one.
   *
   * **Its own operation, asked for rather than paid for.** `get_job` is fetched
   * on every open of a Job; a history has no bound — it grows for as long as
   * the Job lives, and a retried step is a row per attempt plus the moves
   * around it. So the surface that draws it says when it wants one.
   */
  private readonly history: JobReader<{ moves: Recorded[] }>;

  constructor(wiring: JobFocusWiring) {
    this.wiring = wiring;
    this.watched = new JobReader<{ detail: JobDetail }>({
      route: (jobId) => `/jobs/${encodeURIComponent(jobId)}`,
      keeps: (body) => ({ detail: body as JobDetail }),
      keepsLastGood: true,
      // `readAt` moves only where a reading did, so a failure leaves the screen
      // saying when what it shows was last current.
      publish: (watched) =>
        wiring.publish(
          watched.state === "read" ? { watched, readAt: wiring.now() } : { watched },
        ),
    });
    this.resources = new JobReader<{ resources: JobResources }>({
      route: (jobId) => `/jobs/${encodeURIComponent(jobId)}/resources`,
      keeps: (body) => ({ resources: body as JobResources }),
      // A blanked panel reads as a Job holding nothing, which is the exact
      // answer this exists to make loud. The instant on the kept reading is
      // what says how old it is.
      keepsLastGood: true,
      publish: (resources) => wiring.publish({ resources }),
    });
    this.history = new JobReader<{ moves: Recorded[] }>({
      // **The rows are carried, never folded.** `crates/store/src/fold.rs` owns
      // the machine, and Fleet loads the Job before it reads the log — so a
      // history that arrives is one the machine already admitted, and a second
      // fold here would agree with the first only until one of them changed.
      route: (jobId) => `/jobs/${encodeURIComponent(jobId)}/events`,
      keeps: (body) => ({ moves: (body as JobHistory).moves }),
      publish: (history) => wiring.publish({ history }),
    });
  }

  /** The open Job's own id, or `null` — what tells an event's Job from another's. */
  watchedJobId(): string | null {
    return this.watched.jobId;
  }

  /** Read one Job whole and keep it current, or `null` to stop. */
  async watchJob(jobId: string | null): Promise<void> {
    // A footprint belongs to the Job it was read from. Carrying one into the
    // next Job opened would draw another Drone's files under this Job's title.
    const footprint = this.wiring.current().footprint;
    if (footprint.state === "read" && footprint.jobId !== jobId) {
      this.wiring.publish({ footprint: { state: "none" } });
    }
    // And the moment a Drone handed in, for the same reason one line up: it is
    // one Job's, and carried into the next would say a step submitted that has
    // not. `#813`.
    const handed = this.wiring.current().handed;
    if (handed.state === "heard" && handed.jobId !== jobId) {
      this.wiring.publish({ handed: { state: "none" } });
    }
    await this.watched.want(this.wiring.port(), jobId);
  }

  /**
   * Read what the open Job holds on this machine, or `null` to stop.
   *
   * **An examination for another Job is dropped here**, not kept until the
   * next press: a verdict drawn under the wrong title is worse than none, and
   * this is the one place that knows the open Job changed.
   */
  async readResources(jobId: string | null): Promise<void> {
    const found = this.wiring.current().examination;
    if (found.state !== "none" && found.jobId !== jobId) {
      this.wiring.publish({ examination: { state: "none" } });
    }
    await this.resources.want(this.wiring.port(), jobId);
  }

  /**
   * Go and look at this Job now. **The rung below intervene**, and the one act
   * here that moves nothing — what it leaves is a line in the Job's own log.
   *
   * The answer is published rather than returned, so a window that reloaded
   * while a look was out still draws it. The reading beside it is re-read on
   * the same press, because the panel and the verdict must not be two instants.
   */
  async examineJob(jobId: string): Promise<void> {
    const port = this.wiring.port();
    if (port === null) {
      this.wiring.publish({
        examination: { state: "failed", jobId, outcome: { ok: false, why: "not_connected" } },
      });
      return;
    }
    this.wiring.publish({ examination: { state: "looking", jobId } });
    const answer = await ask(port, "POST", `/jobs/${encodeURIComponent(jobId)}/examine`);
    // The open Job moved while the look was out. Nobody has this answer's Job
    // open, and publishing it would draw a verdict under another Job's title.
    if (this.resources.jobId !== jobId) return;
    this.wiring.publish(
      answer.ok === true
        ? { examination: { state: "found", jobId, examined: answer.body as JobExamined } }
        : { examination: { state: "failed", jobId, outcome: answer.outcome } },
    );
    await this.resources.again(port);
  }

  /** Read one Job's transition history, or `null` to stop. */
  async readHistory(jobId: string | null): Promise<void> {
    await this.history.want(this.wiring.port(), jobId);
  }

  /** Re-read the open Job, where the event was about it. */
  refresh(port: number, jobId: string): void {
    void this.takeAgain(port, { because: "job_moved", jobId });
  }

  /**
   * The open Job's screen, brought back whole. **`screen.ts` owns which reads
   * each occasion takes and why**, and it holds none of them — every region is
   * handed in from here, so the list cannot drift from what is actually open.
   */
  takeAgain(port: number, again: Again): Promise<void> {
    return takeAgain(port, again, {
      detail: this.watched,
      resources: this.resources,
      history: this.history,
      turns: this.wiring.turns,
      notes: this.wiring.notes,
      observing: this.wiring.observing(),
      reading: this.wiring.reading(),
      review: this.wiring.review,
    });
  }

  close(): void {
    this.history.close();
  }
}
