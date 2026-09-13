// A person's own edits to a Job's plan: adding a task from the Plan region's
// head, and dropping one with a reason from the row it is on. `#897`;
// `docs/concepts/plan.md`.
//
// Beside `connection.ts` rather than inside it, `rehearsal.ts`'s own reason: a
// POST under a Job answering with the plan it leaves is neither the socket,
// the runtime file nor the state machine. **Its own file rather than a member
// of `JobCommands`**, because `command.ts` sits at the 900-line rule and
// nothing here needs `Board`'s wider surface — a port, and one way to fold
// the plan an answer carries straight into the open Job's own detail, which
// is `job-focus.ts`'s `foldPlan` and not `command.ts`'s `fold`.

import type { AddTask, DropTask, WorkPlan } from "@armada/protocol";
import type { PlanEditAnswer } from "@armada/screens/src/plan-edits";
import { ask, route } from "./request";

/** What adding or dropping a task needs of the connection, and no more. */
export type PlanBoard = {
  port: () => number | null;
  /**
   * The plan a person's own add or drop leaves, folded straight into the open
   * Job's detail where this is it. `job-focus.ts`'s `foldPlan`.
   */
  foldPlan: (jobId: string, plan: WorkPlan) => void;
};

/**
 * `add_task` and `drop_task`. Each POSTs under one Job and answers with the
 * plan the change leaves — folded in at once, so the Plan region redraws
 * without waiting on `job.plan_changed`'s own re-read, which still lands
 * behind it because a working step may act on the plan too.
 *
 * **One set each, not one shared.** A add in flight and a drop in flight are
 * two different presses a person could make on the same Job at once — the
 * head's button and a row's — and a shared set would refuse the second for a
 * press it never made.
 */
export class PlanEdits {
  private readonly board: PlanBoard;
  private readonly adding = new Set<string>();
  private readonly dropping = new Set<string>();

  constructor(board: PlanBoard) {
    this.board = board;
  }

  async add(jobId: string, add: AddTask): Promise<PlanEditAnswer> {
    if (add.title.trim() === "") return { ok: false, outcome: { ok: false, why: "empty_task_title" } };
    if (this.adding.has(jobId)) {
      return { ok: false, outcome: { ok: false, why: "already_adding_task" } };
    }
    const port = this.board.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    this.adding.add(jobId);
    try {
      const answer = await ask(port, "POST", route(jobId, "add_task"), add);
      if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
      const plan = answer.body as WorkPlan;
      this.board.foldPlan(jobId, plan);
      return { ok: true, plan };
    } finally {
      this.adding.delete(jobId);
    }
  }

  async drop(jobId: string, drop: DropTask): Promise<PlanEditAnswer> {
    if (drop.reason.trim() === "") {
      return { ok: false, outcome: { ok: false, why: "empty_task_reason" } };
    }
    if (this.dropping.has(jobId)) {
      return { ok: false, outcome: { ok: false, why: "already_dropping_task" } };
    }
    const port = this.board.port();
    if (port === null) return { ok: false, outcome: { ok: false, why: "not_connected" } };
    this.dropping.add(jobId);
    try {
      const answer = await ask(port, "POST", route(jobId, "drop_task"), drop);
      if (answer.ok !== true) return { ok: false, outcome: answer.outcome };
      const plan = answer.body as WorkPlan;
      this.board.foldPlan(jobId, plan);
      return { ok: true, plan };
    } finally {
      this.dropping.delete(jobId);
    }
  }
}
