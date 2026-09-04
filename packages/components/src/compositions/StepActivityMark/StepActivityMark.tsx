import { Check, CircleDashed, CircleDot, Eye, Flag, Power, RotateCw, X, type LucideIcon } from "lucide-react";

/**
 * Step activity mark — the glyph one rail row carries, and the single place
 * the step-activity vocabulary is written down.
 *
 * Step activity answers **where the work is**, one level below the Job badge.
 * Six of these are `job_steps.state` values; `failed` and `killed` are not — a
 * refusal lands in `last_verdict`, and the split is the point: a step retrying
 * after a refusal is `running` in activity and `failed` in verdict at the same
 * moment, so one column cannot say both. `killed` is the Job's status showing
 * through, freezing the step where it stood.
 *
 * The glyphs are borrowings, under `[conventions.step_activity_borrowing]` in
 * `packages/icons/icons.toml` — a step carries the Job glyph that means the
 * same thing one level down, because a rail row and the badge above it stating
 * the same claim must show the same mark. Two values are not borrowings:
 * `flag` for `stopped`, reserved to it alone, and `circle-dashed` for
 * `not_started`, minted because every borrowing claims something happened.
 *
 * Hue lives in the stylesheet beside the geometry, keyed off `data-activity`.
 * That is the split `Badge` uses, and it keeps one file to read when a value's
 * treatment changes.
 */
export type StepActivity =
  | "not_started"
  | "running"
  | "awaiting_human"
  | "retrying"
  | "advanced"
  | "stopped"
  | "killed"
  | "failed";

/**
 * Activity to glyph. The roster is the registry's, not this file's — every
 * entry here names a glyph with a table in `packages/icons/icons.toml`.
 *
 * `not_started` takes `circle-dashed`, minted for this value alone rather
 * than borrowed. It is a fallback, never a first choice: a step's position is
 * more informative than a silhouette, so wherever a caller supplies an
 * `ordinal` — `WorkflowRail` and `StepRow` both do today — the number wins
 * and the glyph never displaces it. The glyph draws only where no ordinal
 * reaches this component.
 *
 * `awaiting_human` takes `eye`. The registry's borrowing convention lists
 * `eye` and refuses `clock`, and the hue agrees: `--step-waiting` aliases
 * `--status-awaiting-review`, whose badge is `eye`. `clock` is `queued`'s
 * badge and would state a claim the hue does not. Iconography's prose said
 * `clock` here until the two were reconciled; both now say `eye`.
 *
 * `killed` borrows `power`, the Job badge for the same decision one level
 * down. It is on the roster, which was short of it while this file drew it.
 */
const GLYPH: Record<StepActivity, LucideIcon> = {
  not_started: CircleDashed,
  running: CircleDot,
  awaiting_human: Eye,
  retrying: RotateCw,
  advanced: Check,
  stopped: Flag,
  killed: Power,
  failed: X,
};

/**
 * The two activity values that carry a **row surface** rather than a glyph hue
 * alone. A glyph only holds while its row is selected, and the row that ended
 * the Job has to stay findable while you read the Check output beside it. In a
 * rail, background states what the row is and the accent left edge states
 * which row you are on: the surface is constant, selection adds the edge.
 *
 * One of each per rail, because a Job stops or fails in exactly one place.
 * `WorkflowRail` reads this rather than holding a second copy of the rule.
 */
export const STEP_ACTIVITY_CARRIES_A_SURFACE: readonly StepActivity[] = ["stopped", "failed"];

/** Step marks are 12px at strokeWidth 2 — an exact half of lucide's 24 grid. */
const MARK_ICON = 12;
const MARK_STROKE = 2;

export type StepActivityMarkProps = {
  activity: StepActivity;
  /**
   * The accessible name for the mark. Hue and silhouette are the visible
   * channels; this is the third, and it is not optional — a mark whose only
   * label is its colour says nothing to a screen reader.
   */
  label: string;
  /**
   * The step's 1-based position, shown in place of a glyph on `not_started`.
   * A step that has not run has no state to depict, and its number is the one
   * fact about it worth carrying.
   */
  ordinal?: number;
  /**
   * The running mark, still working. One per screen, on the most specific mark
   * present and on the thing being read: a rail's current step, never a step
   * bar and never a second row. Only `running` animates.
   */
  pulsing?: boolean;
};

export function StepActivityMark({ activity, label, ordinal, pulsing = false }: StepActivityMarkProps) {
  const Icon = GLYPH[activity];
  // Only the running mark pulses. Motion carries "still working", which is a
  // claim no other activity value makes.
  const animates = pulsing && activity === "running";
  // `circle-dashed` is a fallback for `not_started` alone, not a first
  // choice: an ordinal is more informative than a silhouette, so it wins
  // wherever a caller supplies one. Every other activity's glyph is a claim
  // about what happened, and an `ordinal` a caller passes alongside it (as
  // `WorkflowRail` does for every row) is not a competing rendering — that
  // glyph always wins, unchanged from before this value had one.
  const showOrdinal = activity === "not_started" && ordinal !== undefined;
  return (
    <span className="armada-step-mark" data-activity={activity} data-pulsing={animates || undefined}>
      {showOrdinal ? (
        <span className="armada-step-mark__ordinal" aria-hidden>
          {ordinal}
        </span>
      ) : (
        <Icon size={MARK_ICON} strokeWidth={MARK_STROKE} aria-hidden />
      )}
      <span className="armada-step-mark__name">{label}</span>
    </span>
  );
}
