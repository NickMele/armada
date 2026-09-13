// What a person's own add or drop of a task answers with. `#897`;
// `docs/concepts/plan.md` owns who may change a Plan and how.

import type { AddTask, DropTask, Outcome, WorkPlan } from "@armada/protocol";

export type { AddTask, DropTask };

/**
 * `POST .../add_task` or `.../drop_task`, read into the app.
 *
 * **The plan it leaves rides on success.** `crates/ipc/operations.toml`'s
 * `add_task` and `drop_task` both answer with the whole `WorkPlan` the change
 * leaves, so the Plan region redraws from the answer at once rather than
 * waiting on `job.plan_changed`'s own re-read — which still lands behind it,
 * because a working step may act on the plan too.
 *
 * `ManifestSpendRead` in `editing.ts` is the shape this follows: `ok` wraps a
 * read that is not a plain `Outcome`, and a refusal is `Outcome` unchanged so
 * every existing refusal reads the same way here as everywhere else.
 */
export type PlanEditAnswer = { ok: true; plan: WorkPlan } | { ok: false; outcome: Outcome };
