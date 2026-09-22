// A Job while somebody is still deciding what it will be. Draft, for
// `crates/ipc/src/detail.rs` and `crates/ipc/src/limits.rs`.
//
// Source of truth today: `JobDetail` at `awaiting_approval`, `StepDetail`'s
// `advance_gate`, `checks`, `judge_checks` and `overridden`, `JobSummary.model`,
// and `LimitValues.concurrency` — "Drones at once", `settings.concurrency-cap`.
//
// **Classifying is the same status as #1159's `proposing`** (#1530, 21 Sep),
// and the workflow and the gates lock at **approval**, not at creation. So this
// is a live shape, editable right up to the approve press.
//
// **A proposal in flight is not a Job today**, which makes everything on the
// proposing screens draft twice over: the shape is draft, and so is the idea
// that it exists before a Job does.

import type { JobDetail, LimitValues, StepDetail } from "@armada/protocol";

import type { PrMode } from "./landing";
import type { TaskTier } from "./task";

/** Which repository policy a step defers to, where it defers to one. */
export type RepositoryDecides = "auto_merge" | "review_gate";

/**
 * What one step is gated by.
 *
 * **Four states per step, not three bits** (#1532, 22 Sep). Three tick boxes
 * cannot express a repository rule, so the fourth state is part of the type
 * rather than a flourish on the control — and it is not new to the wire:
 * `advance_gate` already carries `manifest_rule:auto_merge` and
 * `manifest_rule:review_gate` beside `auto`, `auto_if_judge_passes` and
 * `human_always`.
 */
export type GateView = {
  step_id: string;
  /** Whether the step's Checks are run at its gate. */
  checks: boolean;
  /** Whether the Judge looks. */
  judge: boolean;
  /** Whether it stops for you. */
  you: boolean;
  /** Which repository policy decides, where one does. Absent is the Job's own gate. */
  repository_decides?: RepositoryDecides;
  /** Whether this Job overrode the repository's rule for itself. */
  overridden?: boolean;
};

/** Which model each tier runs on. `null` is Auto — the harness chooses. */
export type TierModels = Readonly<Record<TaskTier, string | null>>;

/** A Job being classified, with everything that locks at approval. */
export type ProposalView = {
  /** `classifying`, and the same status #1159 calls `proposing`. */
  status: string;
  title: string;
  gates: GateView[];
  /**
   * The line the ticks cannot turn off.
   *
   * **A step with nothing ticked still has Fleet checking the work stayed
   * inside the plan** (#1530, 22 Sep). It is `true` and never anything else —
   * a field rather than prose, so a screen cannot draw a step as ungated.
   */
  fleet_always_looks: true;
  tiers: TierModels;
  /**
   * How many Drones this Job may run at once. Absent is "as many as the
   * machine allows".
   */
  drone_cap?: number;
  /**
   * How many the machine allows, across every Job.
   *
   * **This is `LimitValues.concurrency`, and nothing on the wire is called
   * `machine_cap`.** The unit is the open question: `concurrency` is
   * documented as Drones at once and Fleet runs one Drone per Job, so Jobs and
   * Drones are the same count today. #1530 leaves recounting it in Drones open.
   */
  machine_cap: number;
  /** Where the work starts. See `LandingRule.from_ref`. */
  from_ref: string | null;
  pr_mode: PrMode;
  /** When it was approved. Absent is a proposal still being classified. */
  approved_at?: string;
};

/**
 * A Job at its approval gate, as the classifying screen draws it.
 *
 * **`notes_for_planner` is not here.** It was dropped on 22 Sep 2026 — what
 * you want to say goes in the prompt.
 */
export function proposalViewOf(
  detail: JobDetail,
  limits: LimitValues,
  fromRef: string | null = null,
): ProposalView {
  const view: ProposalView = {
    status: detail.job.status,
    title: detail.job.title,
    gates: detail.steps.map(gateViewOf),
    fleet_always_looks: true,
    // Per-tier models are the draft's own. Today one model is resolved for the
    // whole Job, and every task runs on it — so all three tiers name it rather
    // than reading `null`, which would say the harness was choosing.
    tiers: {
      difficult: detail.job.model,
      medium: detail.job.model,
      easy: detail.job.model,
    },
    machine_cap: limits.concurrency,
    from_ref: fromRef,
    pr_mode: "ready",
  };
  if (detail.job.started_at !== undefined) {
    view.approved_at = detail.job.started_at;
  }
  return view;
}

/**
 * One step's gate, from its `advance_gate`.
 *
 * **Absent `advance_gate` is "Fleet cannot say"** — a Job naming a workflow
 * this Fleet does not hold — and it is not an ungated step. It reads as
 * stopping for a person, which is the answer that cannot advance work nobody
 * looked at.
 */
export function gateViewOf(step: StepDetail): GateView {
  const gate = step.advance_gate;
  const view: GateView = {
    step_id: step.step_id,
    checks: (step.checks?.length ?? 0) > 0,
    judge: gate === "auto_if_judge_passes" || (step.judge_checks?.length ?? 0) > 0,
    you: gate === undefined || gate === "human_always",
  };
  if (gate === "manifest_rule:auto_merge") {
    view.repository_decides = "auto_merge";
  }
  if (gate === "manifest_rule:review_gate") {
    view.repository_decides = "review_gate";
  }
  if (view.repository_decides !== undefined) {
    view.overridden = step.overridden;
  }
  return view;
}

/**
 * What one gate is on the wire, and what Fleet does with it.
 *
 * **The boxes are not the wire and the wire is not the boxes.** `advance_gate`
 * is one value per step and says what it takes to *advance*; Checks and a
 * Judge are declarations on the step that run whatever the gate is
 * (`crates/core-model/src/job/declared.rs`). So three independent boxes cover
 * five wire values unevenly, and two of the eight combinations land on a value
 * Fleet cannot act on yet. `unmeant` is where that happens, said rather than
 * hidden behind a tick that looks like it did something.
 */
export type GateReading = {
  /** The `advance_gate` this combination is today. */
  advance_gate: string;
  /** What Fleet does with it, in one sentence. */
  does: string;
  /** Whether Fleet has nothing to do with this combination yet. */
  unmeant: boolean;
};

/** What the two `manifest_rule` keys decide, in the repository's own terms. */
const REPOSITORY_DOES: Readonly<Record<RepositoryDecides, string>> = {
  auto_merge:
    "The repository's auto_merge policy decides whether this lands without a person.",
  review_gate: "The repository's review_gate policy decides whether a person signs off.",
};

/**
 * One step's gate, read as what Fleet would do.
 *
 * The order is the order the wire resolves in: a repository rule nobody
 * overrode is the whole answer, then a person, then the two automatic tiers.
 * **A person outranks the other two boxes rather than replacing them** —
 * `HumanAlways` still runs the tiers, and what they establish is the material
 * the person reads.
 */
export function gateReadingOf(gate: GateView): GateReading {
  if (gate.repository_decides !== undefined && gate.overridden !== true) {
    return {
      advance_gate: `manifest_rule:${gate.repository_decides}`,
      does: REPOSITORY_DOES[gate.repository_decides],
      unmeant: false,
    };
  }
  if (gate.you) {
    return {
      advance_gate: "human_always",
      does: ranBeside(
        "It holds at awaiting_review for you to answer",
        gate.checks,
        gate.judge,
      ),
      unmeant: false,
    };
  }
  if (gate.judge) {
    return {
      advance_gate: "auto_if_judge_passes",
      does: gate.checks
        ? "Its Checks have to pass and the Judge has to decline to refuse them."
        : // `AutoIfJudgePasses` is "the mechanical tier holds **and** the Judge
          // did not refuse", and the enum's own words are that there is no such
          // thing as a Judge pass. A step with no Checks has no mechanical pass
          // for a Judge to decline, so Fleet advances it on nothing.
          "Nothing yet — a Judge declines to refuse a mechanical pass, and this step has none to produce one.",
      unmeant: !gate.checks,
    };
  }
  return {
    advance_gate: "auto",
    does: gate.checks
      ? "Its Checks are the whole gate: they pass and it advances."
      : "Nothing stops it.",
    unmeant: false,
  };
}

/** What still runs on a step a person answers, where anything does. */
function ranBeside(said: string, checks: boolean, judge: boolean): string {
  const ran = [checks ? "its Checks" : undefined, judge ? "the Judge" : undefined].filter(
    (one) => one !== undefined,
  );
  return ran.length === 0
    ? `${said}, with nothing run before you read it.`
    : `${said}, with ${ran.join(" and ")} run first so you read what they found.`;
}

/**
 * The line the ticks cannot turn off.
 *
 * **Fleet refuses work that went outside what the plan declared and looks for
 * a gamed check whatever the boxes say** (#1530, 22 Sep) —
 * `crates/fleet/src/gate.rs`. It is drawn once per screen rather than once per
 * step, because it is the same sentence about every one of them.
 */
export const FLEET_ALWAYS_LOOKS =
  "Whatever is ticked, Fleet checks that the work stayed inside what the plan declared, and " +
  "looks for a Check that was gamed. No tick turns that off.";
