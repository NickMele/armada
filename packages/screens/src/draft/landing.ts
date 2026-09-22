// What a Job does with the work when it is finished. Draft, for
// `crates/ipc/src/configured.rs` and `crates/ipc/src/detail.rs`.
//
// Source of truth today: the Manifest's `base` — "the branch worktrees are cut
// from, where the Manifest names one" — and `JobDelivery`, which records one
// commit, one push and one pull request per Job. Everything that makes landing
// a *choice* rather than a fixed behaviour is the draft's own.
//
// **`ManifestConfig` has no TypeScript mirror.** `crates/ipc/src/configured.rs`
// is where Fleet resolves a Manifest, `base` included, and no type in
// `@armada/protocol` restates it — so the only `base` Bridge can read today is
// `ManifestDeclared.base` off the manifest editor's own read (`amending.ts`).
// That is what this derivation takes, and the gap is worth closing before a
// landing control ships.
//
// **`from_ref` and `target` are both here and they are not the same field**
// (#1530, 22 Sep). They differ when you start from an unmerged branch or land
// in a long-lived one; today's Fleet cuts from the base and lands in the base,
// so the derivation fills both from the one value.

import type { ManifestDeclared } from "@armada/protocol";

/** Whether the unit of a pull request, or of a branch, is the Job or the group. */
export type LandingUnit = "job" | "group";

/** Whether the pull request is offered for review or parked as a draft. */
export type PrMode = "ready" | "draft";

/** What has to happen before the Job counts as finished. */
export type CompleteWhen =
  | "pr_merged"
  | "all_members_landed"
  | "pr_opened"
  | "delivered";

/**
 * Which of those four Fleet can actually answer.
 *
 * **`pr_merged` is not served, and that is a decision rather than a gap**
 * (#1532, 22 Sep): the repository's auto-merge policy decides at the gate —
 * `advance_gate: manifest_rule:auto_merge` — and nothing writes the result of
 * that onto the Job's record. A control offering it would be a setting nothing
 * reads.
 *
 * `JobDelivery.landed` does say a pull request merged, so this is about what
 * completes a *Job*, not about what a surface can observe.
 */
export const COMPLETE_WHEN_SERVED: Readonly<Record<CompleteWhen, boolean>> = {
  pr_merged: false,
  all_members_landed: false,
  pr_opened: false,
  delivered: true,
};

/** How a Job's work reaches the repository. */
export type LandingRule = {
  /**
   * The branch the work lands in.
   *
   * **`null` is the Manifest naming none**, which `amending.ts` already spells
   * that way — "`null` removes the key, and Armada infers a base". It is not
   * the default branch by another name, and a control must not print one.
   */
  target: string | null;
  /** Where the work starts. `null` reads as `target`, on the same rule. */
  from_ref: string | null;
  /** One pull request per Job, or one per group. */
  prs: LandingUnit;
  /** One branch per Job, or one per group. Per-task branches are dropped (#1530). */
  branching: LandingUnit;
  pr_mode: PrMode;
  complete_when: CompleteWhen;
  /**
   * Groups that have to land in one go, each inner list one such set.
   *
   * **"Land together" is a landing setting and not a shape** — the earlier
   * owner decision carried in the design doc. Empty is every group landing on
   * its own.
   */
  land_together: string[][];
};

/**
 * Today's Fleet: one branch, one pull request, cut from the Manifest's base and
 * landing back in it, offered ready, and done when the delivering step has
 * delivered.
 *
 * `manifest` absent is a Job whose Manifest has not been read — both refs are
 * `null`, which a surface says in words rather than drawing a branch name
 * nobody chose.
 */
export function landingRuleOf(manifest?: ManifestDeclared): LandingRule {
  const base = manifest?.base ?? null;
  return {
    target: base,
    from_ref: base,
    prs: "job",
    branching: "job",
    pr_mode: "ready",
    complete_when: "delivered",
    land_together: [],
  };
}
