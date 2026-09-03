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

import type { BridgeState } from "../shared/bridge";
import type { ClearOutcome, Draft, Outcome } from "@armada/protocol";
import type { ChosenAnswer, FileReport, JobSummary, Overruled, ProposeJob, Redirection, Redispatched, Report } from "@armada/protocol";
import type { Proposed } from "@armada/protocol";
import { ask, isJobSummary, MODEL_CALL_MS, route, type Answer } from "./request";
import { Clearing } from "./clearing";
import { proposeFromRequest as propose } from "./proposing";
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
  /**
   * A Job `forget_job` answered for. **Removed, not folded** — there is no
   * row left to replace it with, unlike every other act here.
   */
  forget: (jobId: string) => void;
  /** The whole board again, where what came back was not a row to fold. */
  reread: (port: number) => Promise<void>;
  /** The open Job and its history again, where the act was about that Job. */
  refresh: (port: number, jobId: string) => void;
  publish: (change: Partial<BridgeState>) => void;
};

/**
 * The refusal a second press answers with, **for the acts that go through
 * `act`**. `clearing.ts` keeps its own two: neither of those uses this helper,
 * so listing them here would say a press this file cannot make is refused here.
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
  | "already_overruling"
  | "already_rereading"
  | "already_reporting"
  | "already_deciding"
  | "already_answering";

/**
 * The acts, and which of them are in flight.
 *
 * **One press sends one act, per Job.** The sets are separate rather than one,
 * because killing a Job while a decision is in flight on another is ordinary
 * and a single set would refuse it.
 */
export class JobCommands {
  private readonly board: Board;
  /**
   * The record and the disk, once a Job has ended. **Not acts on a Job**, which
   * is what everything else here is — see `clearing.ts` for the seam.
   */
  private readonly clearing: Clearing;
  private readonly approving = new Set<string>();
  private readonly redispatching = new Set<string>();
  private readonly killing = new Set<string>();
  private readonly redirecting = new Set<string>();
  private readonly restarting = new Set<string>();
  /** Jobs with an override in flight. Its own set: it is its own act. */
  private readonly overruling = new Set<string>();
  /**
   * Jobs with a gate re-run in flight. Its own set beside the override's: the
   * two acts answer triggers that do not overlap, so one set would refuse a
   * press that names a different Job's different act.
   */
  private readonly rereading = new Set<string>();
  /** Jobs with a decision on the work in flight. One press sends one decision. */
  private readonly deciding = new Set<string>();
  /** Jobs with a report being filed. Its own set: filing is not an act on the job. */
  private readonly reporting = new Set<string>();
  /**
   * Jobs with an answer in flight. Its own set beside the redirect's: both put
   * a turn into the same session, and one set would refuse a redirect sent
   * while an answer was still going — which is exactly the moment a person
   * realises none of the options was right.
   */
  private readonly answering = new Set<string>();

  constructor(board: Board) {
    this.board = board;
    this.clearing = new Clearing(board);
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

  /**
   * The other way a Job reaches the same gate: describe the work, and the Job
   * proposer reads it and fills the workflow in.
   *
   * **Not `Outcome`, and not the shape `act` holds.** A plan is every Job the
   * request became, so there is no one row to fold and no one id to hand back —
   * and the request being declined and the call failing are different statuses
   * because a person does different things about them. `Proposed` is both.
   *
   * `proposing.ts` is what it is, for the reason `decide` is `review.ts`'s.
   */
  async proposeFromRequest(request: string): Promise<Proposed> {
    return propose(this.board, request);
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

  // ---------------------------------------------------- clearing up after one
  /**
   * Delete every terminal Job's whole record. **Real deletion, and there is no
   * undo** — `clearing.ts` holds it, with the reclaim beside it.
   */
  clearTerminalJobs(jobIds: readonly string[]): Promise<ClearOutcome> {
    return this.clearing.clearTerminal(jobIds);
  }

  /**
   * Give one terminal Job's worktree and branch back while Fleet is running.
   * **The record survives**, which is the whole of what separates it from the
   * act above.
   */
  reclaimWorktree(jobId: string): Promise<Outcome> {
    return this.clearing.reclaim(jobId);
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
   * Answer the question the job's drone asked. **The job comes back
   * unchanged** — it was `running` while it waited and is `running` now; what
   * moved is the drone, handed the answer as a turn.
   *
   * Nothing is validated here beyond emptiness. Which labels were offered is
   * fleet's, read off a working slot no window holds, so a label this window
   * believes in and fleet does not is a 409 rather than a guess.
   */
  async answerQuestion(
    jobId: string,
    questionId: string,
    chose: string,
  ): Promise<Outcome> {
    if (chose.trim() === "") return { ok: false, why: "empty_instruction" };
    const body: ChosenAnswer = { question_id: questionId, chose };
    return this.act(jobId, this.answering, "already_answering", (port) =>
      ask(port, "POST", route(jobId, "answer_question"), body),
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

  /**
   * Overrule a Judge that refused the work. **The step advances still carrying
   * its failure** — nothing about the verdict is rewritten, and the Job goes on
   * at the next step, or commits and delivers where the refused step was the
   * last one.
   *
   * Its own act rather than a mode on `approveReview`: that one answers
   * `awaiting_review`, this one answers `escalated`, and one call taking which
   * would let a refusal be taken with the act built for work nobody objected
   * to. Blank is refused before the request is sent, matching the 422 Fleet
   * would give it — and refused at all because an override that says nothing
   * gives the rate and never the cause.
   */
  async overrideVerdict(jobId: string, reason: string): Promise<Outcome> {
    if (reason.trim() === "") return { ok: false, why: "empty_reason" };
    const body: Overruled = { reason };
    return this.act(jobId, this.overruling, "already_overruling", (port) =>
      ask(port, "POST", route(jobId, "override_verdict"), body),
    );
  }

  /**
   * Ask the gate again, over the evidence the step already submitted.
   *
   * **It re-runs; it does not retry.** No Drone works, nothing the Drone did is
   * redone, and the step's retry budget is untouched — the run the gate is told
   * it is reading is the one the Drone worked. What comes back is the Job, and
   * a second undecided reading answers with a Job that did not move, which is
   * the honest outcome rather than a refusal.
   *
   * **No body, and that is the act rather than an omission.** An override
   * carries a required reason because a person is disagreeing with a machine.
   * Nothing ruled here, so there is nothing to disagree with and no sentence
   * the second reading will not say for itself.
   */
  async rerunGate(jobId: string): Promise<Outcome> {
    return this.act(jobId, this.rereading, "already_rereading", (port) =>
      // The Judge is asked inside this request, so it waits on a model call and
      // not on a store read. `MODEL_CALL_MS` says why the ordinary wait is the
      // wrong one here.
      ask(port, "POST", route(jobId, "rerun_gate"), undefined, MODEL_CALL_MS),
    );
  }

  // ---------------------------------------------------------- reporting
  /**
   * Say this job failed in error, and file its record with the reason.
   *
   * **Not the shape `act` holds, and deliberately.** Every act in that shape
   * folds a job onto the board and re-reads the open one, because every act in
   * that shape changes a job. This one changes nothing — no status, no step, no
   * drone — so folding or re-reading would be Bridge redrawing a board on the
   * strength of somebody having written a sentence.
   *
   * Blank is refused before the request is sent, matching the 422 Fleet would
   * give it. What comes back is the report, which the caller shows: Armada
   * files nothing in the issue tracker, so the rendered record is what a person
   * there themselves.
   */
  async fileReport(jobId: string, filing: FileReport): Promise<Outcome> {
    if (filing.said.trim() === "") return { ok: false, why: "empty_report" };
    if (this.reporting.has(jobId)) return { ok: false, why: "already_reporting" };
    const port = this.board.port();
    if (port === null) return { ok: false, why: "not_connected" };

    this.reporting.add(jobId);
    try {
      const answer = await ask(port, "POST", route(jobId, "report"), filing);
      if (answer.ok !== true) return answer.outcome;
      const report = answer.body as Report;
      // A body that is not a report is still a filing that happened — Fleet
      // answered 201 — so the outcome is a success carrying nothing rather
      // than a refusal that would send somebody to file it a second time.
      return typeof report?.record === "string" ? { ok: true, report } : { ok: true };
    } finally {
      this.reporting.delete(jobId);
    }
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
