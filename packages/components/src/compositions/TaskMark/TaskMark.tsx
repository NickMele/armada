import { Check, CircleDashed, CircleDot, Minus, type LucideIcon } from "lucide-react";

/**
 * Task mark — the glyph a Plan region's row leads with. The mark alone
 * carries a task's state (`docs/journeys/monitor-active-work.md`, Plan); no
 * row prints a state word.
 *
 * `open`/`working`/`done`/`dropped` are `docs/concepts/plan.md`'s own
 * vocabulary, not `job_steps.state`, so there is no `enum-verbs.toml` row to
 * read the accessible names from — `SAID` below is written, not generated.
 *
 * `circle-dashed` and `minus` are each reserved to two uses now —
 * `packages/icons/icons.toml`'s `[[icons.circle-dashed.usage]]` and
 * `[[icons.minus.usage]]` name this component's `open` and `dropped` marks
 * beside the one each glyph already carried.
 */
export type TaskMarkState = "open" | "working" | "done" | "dropped";

const GLYPH: Record<TaskMarkState, LucideIcon> = {
  open: CircleDashed,
  working: CircleDot,
  done: Check,
  dropped: Minus,
};

/** Sentence case, for the accessible name alone — no row prints these. */
const SAID: Record<TaskMarkState, string> = {
  open: "Open",
  working: "Working",
  done: "Done",
  dropped: "Dropped",
};

/** Task marks are 12px at strokeWidth 2, `StepActivityMark`'s own geometry. */
const MARK_ICON = 12;
const MARK_STROKE = 2;

export type TaskMarkProps = {
  state: TaskMarkState;
};

/** Never pulses — the pulse stays scoped to the running step's own mark. */
export function TaskMark({ state }: TaskMarkProps) {
  const Icon = GLYPH[state];
  return (
    <span className="armada-task-mark" data-state={state}>
      <Icon size={MARK_ICON} strokeWidth={MARK_STROKE} aria-hidden />
      <span className="armada-task-mark__name">{SAID[state]}</span>
    </span>
  );
}
