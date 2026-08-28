// Every act Bridge performs on a Job, and the one shape seven of them share.
//
// Beside `connection.ts` rather than inside it, for the reason `observe.ts`,
// `request.ts` and `review.ts` are: the connection is a socket, a runtime file
// and a state machine, and a POST to a route under a Job is none of those. The
// acts grew there because each landed on its own — a redispatch, a redirect and
// a restart, then three review decisions — and by the ninth the file was 700
// lines carrying four copies of the same twenty.
//
// Nothing here holds a connection. It is handed a `Board`, which is the whole
// of what an act needs: where to send, and what to do with the Job that comes
// back. So nothing here can drift from what the socket believes.
//
// **Bridge never talks to a Drone.** Every act below names Fleet — which is
// what makes "kill the Drone" a request to the daemon that spawned it.

import type { BridgeState, Draft, Outcome } from "../shared/bridge";
import type { JobSummary, ProposeJob, Redirection, Redispatched } from "../shared/protocol";
import { ask, isJobSummary, type Answer } from "./request";
import { decide, type Decision } from "./review";

/**
 * What an act needs of the connection, and nothing more.
 *
 * The port is asked for rather than held, because a command sent over a
 * connection that has since dropped is a command with nowhere to go — and it is
 * asked for **once**, at the top of the act, so the re-reads that follow the
 * answer go to the same Fleet the request did.
 */
export type Board = {
  /** Where Fleet answers, or `null` when nothing is connected. */
  port: () => number | null;
  /** A Job a command answered with, onto the board. */
  fold: (job: JobSummary) => void;
  /** The whole board again, where what came back was not a row to fold. */
  reread: (port: number) => Promise<void>;
  /** The open Job and its history again, where the act was about that Job. */
  refresh: (port: number, jobId: string) => void;
  publish: (change: Partial<BridgeState>) => void;
};

/**
 * The refusal a second press answers with.
 *
 * **Written out rather than derived from the route.** Two routes share
 * `already_killing` and three share `already_deciding`, because what is in
 * flight is the act and not the endpoint — a rule that computed one from the
 * other would have to be wrong for one of those groups.
 */
type Busy =
  | "already_killing"
  | "already_redirecting"
  | "already_restarting"
  | "already_deciding";

/** A route under one Job. The id is a path segment, so it is encoded. */
function route(jobId: string, operation: string): string {
  return `/jobs/${encodeURIComponent(jobId)}/${operation}`;
}

/**
 * The acts, and which of them are in flight.
 *
 * **One press sends one act, per Job.** The sets are separate rather than one,
 * because killing a Job while a decision is in flight on another is ordinary
 * and a single set would refuse it.
 */
export class JobCommands {
  private readonly board: Board;
  private readonly approving = new Set<string>();
  private readonly redispatching = new Set<string>();
  private readonly killing = new Set<string>();
  private readonly redirecting = new Set<string>();
  private readonly restarting = new Set<string>();
  /** Jobs with a decision on the work in flight. One press sends one decision. */
  private readonly deciding = new Set<string>();

  constructor(board: Board) {
    this.board = board;
  }

  /**
   * One act on one Job: refuse a second press, POST to a route under the Job,
   * fold the Job that came back, re-read the open one.
   *
   * **Seven acts were this, written out four times** — the two kills, a
   * redirect, a restart and the three review decisions. What differs between
   * them is the request, and which press is already in flight; those are the
   * arguments, and there are no others.
   *
   * **No refusal is flattened here.** `ask` maps what Fleet answered and this
   * hands it back untouched, so a mapping made explicit at the seam stays
   * explicit. `busy` is Bridge's own refusal and is named by the caller, not
   * inferred.
   */
  private async act(
    jobId: string,
    inFlight: Set<string>,
    busy: Busy,
    send: (port: number) => Promise<Answer>,
  ): Promise<Outcome> {
    if (inFlight.has(jobId)) return { ok: false, why: busy };
    const port = this.board.port();
    if (port === null) return { ok: false, why: "not_connected" };

    inFlight.add(jobId);
    try {
      const answer = await send(port);
      if (answer.ok !== true) return answer.outcome;
      // Folding whatever a route answered would put a malformed row on the
      // board, so a body that is not one is a re-read instead.
      if (isJobSummary(answer.body)) this.board.fold(answer.body);
      else await this.board.reread(port);
      this.board.refresh(port, jobId);
      return { ok: true };
    } finally {
      inFlight.delete(jobId);
    }
  }

  // ------------------------------------------------------------- proposing
  /** Draft a Job onto the approval gate. What comes back is not a running Job. */
  async proposeJob(draft: Draft): Promise<Outcome> {
    // Refused here and not only in the form, whose check is a courtesy.
    if (draft.title.trim() === "") return { ok: false, why: "empty_title" };
    if (draft.brief.trim() === "") return { ok: false, why: "empty_brief" };
    // Ids Fleet holds. An empty one is an unfilled form, not a value to send.
    if (draft.workflowId === "") return { ok: false, why: "no_workflow" };
    if (draft.manifestId === "") return { ok: false, why: "no_manifest" };
    const port = this.board.port();
    if (port === null) return { ok: false, why: "not_connected" };

    const proposal: ProposeJob = {
      // Rust stores a trimmed `Title`, so padding makes what comes back
      // differ from what was sent.
      title: draft.title.trim(),
      workflow_id: draft.workflowId,
      owner_manifest_id: draft.manifestId,
      origin: draft.origin,
      urgency: draft.urgency,
      atomic: draft.atomic,
      // Omitted rather than sent empty: `""` reads like a value, and Fleet
      // fills it from configuration.
      ...(draft.model === "" ? {} : { model: draft.model }),
      facts: draft.brief,
      // Sent unconditionally, even empty: unlike `model` there is no
      // meaningful absent-vs-empty reading for Fleet to fill in.
      attachments: draft.attachments.map((attachment) => ({
        staged_path: attachment.path,
        filename: attachment.filename,
        mime_type: attachment.mimeType,
      })),
    };
    const answer = await ask(port, "POST", "/jobs", proposal);
    if (answer.ok !== true) return answer.outcome;
    this.board.fold(answer.body as JobSummary);
    return { ok: true };
  }

  // ------------------------------------------------------------ dispatching
  /**
   * Release a Job to spawn. Approving twice does not spawn twice.
   *
   * **Not the shape `act` holds**, and the reason is the first line of the
   * `try`: this is the one act that publishes its own press, so a row can draw
   * itself as being approved. That it also folds without a shape check and
   * refreshes nothing is how it was written; both are left exactly as they
   * were, because a reduction that changes what an act does to the board is
   * not a reduction.
   */
  async approveDispatch(jobId: string): Promise<Outcome> {
    if (this.approving.has(jobId)) return { ok: false, why: "already_approving" };
    const port = this.board.port();
    if (port === null) return { ok: false, why: "not_connected" };

    this.approving.add(jobId);
    this.board.publish({ approving: [...this.approving] });
    try {
      const answer = await ask(port, "POST", route(jobId, "approve_dispatch"));
      if (answer.ok !== true) return answer.outcome;
      this.board.fold(answer.body as JobSummary);
      return { ok: true };
    } finally {
      this.approving.delete(jobId);
      this.board.publish({ approving: [...this.approving] });
    }
  }

  /**
   * Kill the failed Job and mint its replacement. **Nothing is reopened.**
   *
   * Two Jobs come back because a redispatch is two acts, and both are folded so
   * the board shows the lineage. Accepted only from `escalated`.
   *
   * **Not the shape `act` holds**: two rows come back rather than one, and the
   * id that comes out is not the id that went in.
   */
  async redispatchJob(jobId: string): Promise<Outcome> {
    if (this.redispatching.has(jobId)) return { ok: false, why: "already_redispatching" };
    const port = this.board.port();
    if (port === null) return { ok: false, why: "not_connected" };

    this.redispatching.add(jobId);
    try {
      const answer = await ask(port, "POST", route(jobId, "redispatch"));
      if (answer.ok !== true) return answer.outcome;
      const both = answer.body as Redispatched;
      // Folding whatever a route answered would put a malformed row on the
      // board.
      if (!isJobSummary(both.replaced) || !isJobSummary(both.dispatched)) {
        await this.board.reread(port);
        return { ok: true };
      }
      this.board.fold(both.replaced);
      this.board.fold(both.dispatched);
      // The replacement's id: the Job the caller asked about is over, and the
      // one worth opening did not exist a moment ago.
      return { ok: true, jobId: both.dispatched.id };
    } finally {
      this.redispatching.delete(jobId);
    }
  }

  // ---------------------------------------------------------------- stopping
  /** Kill the Drone. **The Job survives**, its worktree held for a redispatch. */
  async killDrone(jobId: string): Promise<Outcome> {
    return this.kill(jobId, "kill_drone");
  }

  /**
   * End the Job at `killed`, terminal. Legal from every non-terminal status,
   * including those no Drone ran under — which is why it is not the same act,
   * or the same button, as killing one.
   */
  async killJob(jobId: string): Promise<Outcome> {
    return this.kill(jobId, "kill_job");
  }

  /** One in flight per Job covers both kills: a second press aims at a row
   * that has already moved. */
  private kill(jobId: string, operation: "kill_drone" | "kill_job"): Promise<Outcome> {
    return this.act(jobId, this.killing, "already_killing", (port) =>
      ask(port, "POST", route(jobId, operation)),
    );
  }

  // --------------------------------------------------------------- resuming
  /**
   * Say something to the Drone that is there. **The Job comes back
   * `running`**, at the same step, with the same Drone — nothing was spawned
   * and nothing was thrown away. Blank is refused before the request is
   * sent, matching the 422 Fleet would give it.
   */
  async redirectDrone(jobId: string, instruction: string): Promise<Outcome> {
    if (instruction.trim() === "") return { ok: false, why: "empty_instruction" };
    const body: Redirection = { instruction };
    return this.act(jobId, this.redirecting, "already_redirecting", (port) =>
      ask(port, "POST", route(jobId, "redirect"), body),
    );
  }

  /**
   * Put a fresh Drone on the worktree the last one left, at the step that
   * stopped. **One Job comes back**, resuming rather than replacing — the
   * whole difference between this and a redispatch.
   */
  async restartStep(jobId: string): Promise<Outcome> {
    return this.act(jobId, this.restarting, "already_restarting", (port) =>
      ask(port, "POST", route(jobId, "restart_step")),
    );
  }

  // ------------------------------------------------------ deciding on work
  /** Take the work. On the last step Fleet commits and delivers first. */
  async approveReview(jobId: string): Promise<Outcome> {
    return this.settleWork(jobId, "approve_review");
  }

  /** Send it back. **`running` again**, same step, same Drone. Blank refused. */
  async requestChanges(jobId: string, note: string): Promise<Outcome> {
    if (note.trim() === "") return { ok: false, why: "empty_note" };
    return this.settleWork(jobId, "request_changes", note);
  }

  /** A verdict on the work. **Terminal, and it ends the Drone.** */
  async rejectWork(jobId: string): Promise<Outcome> {
    return this.settleWork(jobId, "reject");
  }

  /**
   * One decision, sent once. **One in flight per Job covers all three**: a
   * second press aims at a Job that has already left `awaiting_review`, the
   * only status any of the three is legal on.
   *
   * The route is `review.ts`'s to build, because which of the three it is is
   * the whole of the difference between them.
   */
  private settleWork(jobId: string, what: Decision, note?: string): Promise<Outcome> {
    return this.act(jobId, this.deciding, "already_deciding", (port) =>
      decide(port, jobId, what, note),
    );
  }
}
