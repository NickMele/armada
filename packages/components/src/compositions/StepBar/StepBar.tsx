import { Tooltip } from "../../primitives/Tooltip/Tooltip";
import type { StepActivity } from "../StepActivityMark/StepActivityMark";

/**
 * Step bar — progress on a list row, as a bar rather than a fraction.
 *
 * "4 of 7" has to be read and converted; a bar is read at a glance, and read
 * down a column it shows which Jobs are near the end. One segment per step, so
 * the segment width itself says how long the workflow is. The exact count moves
 * to the tooltip.
 *
 * **The bar never pulses.** Its job is where the work got to, which is a static
 * fact. On a list row the badge carries the pulse, because it sits in the same
 * fixed column on every row and the motion then appears in one predictable
 * place rather than moving with the workflow's length.
 *
 * **A step behind the current one keeps its hue**, `--step-advanced`, the same
 * as in a rail. The bar used to draw them neutral on the argument that a list
 * of Jobs would be a wall of green; the owner ruled the other way on 11 Sep
 * 2026, because a grey run of finished steps read as steps that had not
 * happened.
 *
 * **A second shape draws a Job's tasks on the same segment grammar**, for
 * `#896` and `#898` — see `TaskBarSegment` below. A step bar and a task bar
 * never both apply to one call: the props are a union, and `tasks` replaces
 * `total`/`current`/`activity` rather than sitting beside them.
 */
/**
 * One task's state, as the bar draws it. **Not `StepActivity`.** A step's
 * progress is one position moving through an ordered line — past, current,
 * remaining — and a task's is an independent claim per task: several could
 * read `done` at once, and none of them is "the current one" the way a step
 * is. `open`/`working`/`done` map onto the same three segment weights
 * (`remaining`/`current`/`past`) because the drawing wants one segment
 * grammar, not because a task's state is a position.
 *
 * **`failed` is a claim rather than a weight**, and it is what a boundary's
 * Check takes: given `past` it would read as one of the six beside it that
 * passed. `#1536`.
 */
export type TaskBarSegment = "open" | "working" | "done" | "failed";

export type StepBarProps =
  | {
      /** One segment per step. Segment width is what says how long a workflow is. */
      total: number;
      /**
       * The 1-based position of the current step. `0` for a Job that has not
       * started: every segment is remaining and none takes hue.
       */
      current: number;
      /**
       * The current step's activity, and the hue its segment takes. The steps
       * behind it take `advanced`'s. `killed` and `retrying` take none, as
       * everywhere else.
       */
      activity?: StepActivity;
      tasks?: undefined;
      /**
       * The exact count, as the tooltip carries it. Written by the caller,
       * because "Step 4 of 7" is a sentence and this component composes none.
       */
      label?: string;
    }
  | {
      total?: undefined;
      current?: undefined;
      activity?: undefined;
      /**
       * A Job's tasks, one segment per task not dropped, in plan order.
       * **The extension `docs/contracts/design-system.md` did not have**: a
       * step bar draws one position moving through a line, and a task bar
       * draws several independent claims, so segment state rides in on its
       * own list rather than being derived from `total`/`current`.
       *
       * **`done` keeps `--step-advanced` everywhere**, on the Board and on
       * job detail alike — the Board's own step bar beside it colours an
       * advanced step green, so a neutral task segment would disagree with
       * the field next to it on one row. The neutral list rule is the
       * Active Jobs step bar's own, not this one's.
       */
      tasks: readonly TaskBarSegment[];
      label?: string;
    };

export function StepBar(props: StepBarProps) {
  const { label } = props;
  const segments =
    props.tasks !== undefined
      ? props.tasks.map((task) => ({
          state:
            task === "done"
              ? ("past" as const)
              : task === "working" || task === "failed"
                ? ("current" as const)
                : ("remaining" as const),
          activity:
            task === "working" ? ("running" as const) : task === "failed" ? ("failed" as const) : undefined,
        }))
      : Array.from({ length: props.total }, (_, i) => {
          const position = i + 1;
          const state = position < props.current ? "past" : position === props.current ? "current" : "remaining";
          return {
            state: state as "past" | "current" | "remaining",
            activity: state === "current" ? (props.activity ?? "not_started") : undefined,
          };
        });

  const fallbackLabel =
    props.tasks !== undefined
      ? `${props.tasks.filter((task) => task === "done").length} of ${props.tasks.length} tasks`
      : `Step ${props.current} of ${props.total}`;

  const bar = (
    <span className="armada-step-bar" role="img" aria-label={label ?? fallbackLabel}>
      {segments.map((segment, i) => (
        <span
          key={i}
          className="armada-step-bar__segment"
          data-state={segment.state}
          data-activity={segment.activity}
        />
      ))}
    </span>
  );

  // The exact count lives in the tooltip, which is why the bar carries no
  // number of its own.
  return label ? <Tooltip label={label}>{bar}</Tooltip> : bar;
}
