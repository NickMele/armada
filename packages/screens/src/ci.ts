// The review's Pull request CI row, from what the sweep last read off the pull request. #905.
//
// **The forge's own CI, never totalled with Armada's Checks.** Its words say what ran on the
// forge; a conflict with main outranks a failed run, because a run on a conflicted branch
// says nothing about the change.

import type { ConfidenceCi } from "@armada/components";
import type { PullRequestChecks } from "@armada/protocol";

/** What a person can do from the row, bound to the Job by the caller. */
export type CiActs = Pick<ConfidenceCi, "onInvestigate" | "onRerun" | "onResolve" | "disabled">;

export function ciOf(
  checks: PullRequestChecks | undefined,
  conflicted: boolean,
  acts: CiActs,
): ConfidenceCi {
  return {
    kind: checks?.kind ?? "unreadable",
    said: conflicted ? "Conflicts with main" : saidOf(checks),
    failed: conflicted ? [] : (checks?.failed ?? []),
    conflicted,
    ...acts,
  };
}

/** The row's result, a phrase in the forge's terms: `1 of 4 failed`. */
export function saidOf(checks: PullRequestChecks | undefined): string {
  if (checks === undefined) return "Not read yet";
  switch (checks.kind) {
    case "nothing_ran":
      return "Nothing ran";
    case "all_passed":
      return `${checks.checks} of ${checks.checks} passed`;
    case "still_waiting":
      return `${checks.finished ?? 0} of ${checks.checks} finished, none failed yet`;
    case "some_failed": {
      const failed = checks.failed?.length ?? 0;
      return `${failed} of ${checks.checks} failed`;
    }
    case "unreadable":
      return "The forge would not say what ran";
  }
}

/**
 * Whether the row is drawn at all: a conflict with main, or CI that ran or is being read.
 * **Nothing ran draws nothing**, and neither does a pull request Fleet has not read.
 */
export function ciShown(checks: PullRequestChecks | undefined, conflicted: boolean): boolean {
  return conflicted || (checks !== undefined && checks.kind !== "nothing_ran");
}
