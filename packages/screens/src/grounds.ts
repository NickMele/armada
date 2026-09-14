import type { ConfidenceCaptured, ConfidenceGrounds, GroundRow } from "@armada/components";
import type { Criterion, Diff, StepDetail, Submitted, Work } from "@armada/protocol";
import { onlyCurrentAttempt } from "./facts";
import { shownFrames, type Frames } from "./frames";
import { checksOf, checksStand, didNotPass, howTheChecksWent, panelsOf } from "./gates";

/**
 * `verification::EVIDENCE_SCOPE`. **Written down only when it fails**, so a step at the
 * gate with no such row is one whose changed files the scope Check let through.
 */
const EVIDENCE_SCOPE = "evidence_scope";

/** What the review's verdict rests on, read off the step waiting at the gate. */
export function groundsOf(
  step: StepDetail,
  criteria: readonly Criterion[],
  claim: Submitted | undefined,
  diff: Diff,
): ConfidenceGrounds {
  const checks = checksRowOf(step);
  const judge = judgeRowOf(step, criteria);
  const evidence = evidenceRowOf(step, claim);
  const plan = planRowOf(diff.state === "read" ? diff.work : undefined);
  return {
    ...(checks === undefined ? {} : { checks }),
    ...(judge === undefined ? {} : { judge }),
    ...(evidence === undefined ? {} : { evidence }),
    ...(plan === undefined ? {} : { plan }),
  };
}

/** The step's Checks, counted the way the phase strip counts them. */
export function checksRowOf(step: StepDetail): GroundRow | undefined {
  const reads = checksOf(step);
  if (reads.length === 0) return undefined;
  const { ran, failed } = howTheChecksWent(reads);
  return {
    result: checksStand(reads),
    tone: failed.length > 0 ? "not_met" : ran.length === reads.length ? "met" : "quiet",
    detail: reads.map((read) => read.name).join(", "),
  };
}

/** One row per criterion, and one veto refuses it. */
export function judgeRowOf(step: StepDetail, criteria: readonly Criterion[]): GroundRow | undefined {
  const panels = panelsOf(step, criteria);
  if (panels.length === 0) return undefined;
  const of = `of ${panels.length} ${panels.length === 1 ? "criterion" : "criteria"}`;
  const refused = panels.filter((panel) => panel.verdict === "not_met");
  if (refused.length > 0) {
    return {
      result: `${refused.length} ${of} not met`,
      tone: "not_met",
      detail: refused.map((panel) => panel.criterion?.text ?? panel.criterionId).join(" · "),
    };
  }
  const met = panels.filter((panel) => panel.verdict === "met").length;
  return { result: `${met} ${of} met`, tone: met === panels.length ? "met" : "quiet" };
}

/** Whether the files that changed stayed inside what the step's evidence was allowed to touch. */
export function evidenceRowOf(step: StepDetail, claim: Submitted | undefined): GroundRow | undefined {
  const scope = onlyCurrentAttempt(step.check_runs).find((run) => run.name === EVIDENCE_SCOPE);
  if (scope !== undefined && didNotPass(scope)) {
    return {
      result: "Outside its scope",
      tone: "not_met",
      ...(scope.produced === undefined ? {} : { detail: scope.produced }),
    };
  }
  if (claim === undefined) return undefined;
  return {
    result: "Within scope",
    tone: "met",
    detail: "No changed file is under a path the step excludes or Armada forbids",
  };
}

/**
 * Files the step changed outside the plan it declared. **Drift is the Judge's and does not
 * fail the step**, so this is quiet rather than a failure. Nothing where no plan was declared.
 */
export function planRowOf(work: Work | undefined): GroundRow | undefined {
  if (work === undefined || !work.plan_declared) return undefined;
  const outside = work.files.filter((file) => file.outside_plan === true).map((file) => file.path);
  if (outside.length === 0) return { result: "Inside the plan", tone: "met" };
  return {
    result: outside.length === 1 ? "1 file outside the plan" : `${outside.length} files outside the plan`,
    tone: "quiet",
    detail: outside.map((path) => `\`${path}\``).join(", "),
  };
}

/** The current attempt's frames and the Drone's claim, or nothing where there is neither. */
export function capturedOf(
  step: StepDetail,
  claim: Submitted | undefined,
  frames: Frames,
): ConfidenceCaptured | undefined {
  const kept = onlyCurrentAttempt(step.frames ?? []);
  if (kept.length === 0 && claim === undefined) return undefined;
  return {
    frames: shownFrames(kept, frames),
    ...(claim === undefined
      ? {}
      : {
          claim: {
            claimed: claim.claimed,
            shownBy: claim.shown_by,
            ...(claim.not_claimed === undefined ? {} : { notClaimed: claim.not_claimed }),
          },
        }),
  };
}
