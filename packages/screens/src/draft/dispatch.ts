// What a person may set while they are still typing the request. Draft, for
// `crates/ipc/src/job.rs` — `ProposeFromRequest`, which today carries a
// request, a repository and staged files and nothing else.
//
// Source of truth today: `WorkflowSummary` and `StepSummary.advance_gate` for
// how a Job lands, `JobSummary.model` for the one model a Job runs on,
// `LimitValues.concurrency` for what the machine allows. Every field here is
// **absent by default**, because the board's rule is that you set any of them
// or none (#1540, 22 Sep) — and absent is not a zero or a first option, it is
// the answer being somebody else's.
//
// **`ProposalView` is the same settings one moment later.** That shape is a
// Job being classified, after the proposer has answered; this one is before it
// has been asked, so every field is optional where that one's are resolved.
// `dispatchSettingsOf` is the join, so the two surfaces cannot drift.

import type { WorkflowSummary } from "@armada/protocol";

import type { ProposalView, TierModels } from "./proposal";

/**
 * Who decides that the work has landed.
 *
 * **Two values, because the third is not a setting.** A workflow's delivering
 * step already carries `auto`, `auto_if_judge_passes`, `human_always` and the
 * two `manifest_rule:` gates; what a person sets here is the one bit those
 * five collapse to at dispatch — whether anybody is asked. The rest locks at
 * approval, where the gate rows are (#1530, 21 Sep).
 */
export type LandsWhen = "auto" | "you_at_review";

/**
 * The Settings block, as the dispatch form holds it.
 *
 * **Every field is optional and absent means decided elsewhere** — the
 * proposer picks the workflow, the planner's tier map falls back to the
 * configured model, the machine's own cap holds, and the workflow's delivering
 * step decides how it lands. A form that filled these in with defaults would
 * be a person asserting four things they did not choose.
 */
export type DispatchSettingsView = {
  /** Which workflow to run, where a person overrode the proposer's read. */
  workflow_id?: string;
  /** Which model each tier runs on. `null` inside it is Auto — the harness chooses. */
  tiers?: TierModels;
  /** How many Drones this Job may run at once, inside the machine's own cap. */
  drone_cap?: number;
  lands?: LandsWhen;
};

/** Nothing set. What the form opens on, and what a dispatch sends when nobody touches it. */
export const NOTHING_SET: DispatchSettingsView = {};

/**
 * How many of the four a person has set.
 *
 * **What the block's head says while it is closed.** A collapsed section that
 * cannot say whether anything is inside it is a section people open to check.
 */
export function howManySet(settings: DispatchSettingsView): number {
  return [settings.workflow_id, settings.tiers, settings.drone_cap, settings.lands].filter(
    (one) => one !== undefined,
  ).length;
}

/**
 * The same settings, read off a proposal that already exists.
 *
 * **Absent stays absent.** A proposal carries a tier map and a cap because
 * Fleet resolved them; a `drone_cap` it does not carry is the machine's cap
 * holding, and that is not a value to copy into a person's form.
 */
export function dispatchSettingsOf(proposal: ProposalView): DispatchSettingsView {
  const settings: DispatchSettingsView = { tiers: proposal.tiers };
  if (proposal.drone_cap !== undefined) settings.drone_cap = proposal.drone_cap;
  return settings;
}

/**
 * How a workflow lands, before anybody overrides it.
 *
 * **Read off the step that delivers, not off the last step.** A workflow whose
 * delivering step is not its last is a real shape — `code-review` delivers a
 * review — and reading the last one would say the wrong thing about it. A
 * workflow that delivers nowhere reads `auto`: nothing stops for a person,
 * because nothing is handed over.
 */
export function landsAt(workflow: WorkflowSummary): LandsWhen {
  const delivers = workflow.steps.find((step) => step.delivers);
  return delivers?.advance_gate === "human_always" ? "you_at_review" : "auto";
}
