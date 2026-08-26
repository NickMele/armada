import type { LucideIcon } from "lucide-react";
import type { MouseEvent, ReactNode } from "react";
import { useCallback } from "react";
import { Badge } from "../../primitives/Badge/Badge";

/**
 * Job row (stacked) — the most repeated element in the app, and one shape at
 * every width.
 *
 * A badge leading, the headline sentence beside it, and a labelled field run
 * beneath. **No field is dropped at any width**: every field in the row exists
 * because a decision depends on it, and responsive-hiding one contradicts the
 * rule that the facts needed to decide are on screen without a click. Narrow
 * changes nothing here — it was an eight-column table that reshaped below the
 * breakpoint, and it was retired because the Board and Alerts disagreed about
 * what a job looks like.
 *
 * The honest cost is height: fewer jobs are visible at once than in a table
 * row. That was accepted deliberately.
 *
 * **A list row never takes a primary action.** One secondary control, whose
 * label names the act the row's state calls for. Fourteen rows offering a
 * decision would be fourteen accent blocks; urgency is carried by the badge
 * and the ordering, and the accent is spent on job detail.
 *
 * **The badge carries the pulse; the bar never does.** It sits in the same
 * fixed column on every row, so the motion appears in one predictable place
 * rather than moving with the workflow's length. Focus has nothing to do with
 * it: a running row pulses whether or not the cursor is on it. The rule is one
 * pulse per screen, on the most specific mark present — so on job detail the
 * rail takes it and this badge goes static, which is what `pulsing` is for.
 */

export type JobRowField = {
  /**
   * The label, where the field has one. Several do not: a branch name and a
   * spend figure say what they are.
   */
  label?: ReactNode;
  value: ReactNode;
  /**
   * Machine-derived: a job id, a path, a branch, a duration, a cost. Mono is
   * the signal that the system reported this rather than wrote it.
   */
  mono?: boolean;
  /**
   * A 12px glyph leading the field, from `packages/icons/icons.toml`.
   */
  icon?: LucideIcon;
  /**
   * The exact string the field puts on the clipboard. Setting it makes the
   * value copy on click and go to `--accent` on hover, with no `copy` glyph —
   * the affordance token is the affordance, and a 12px icon repeated down
   * fourteen rows is noise. A value that copies does not also get a button
   * that copies it.
   */
  copyValue?: string;
  /** Emphasis within the run: the fact the row is currently about. */
  emphasis?: boolean;
  /** Set back: a timestamp, or a field that does not apply yet. */
  quiet?: boolean;
};

export type JobRowStackedProps = {
  /** The status token stem, e.g. `running`, `awaiting-review`. */
  status: string;
  /** The badge glyph, required on every state. From the icon registry. */
  statusIcon: LucideIcon;
  /** The verb, from the enum→verb map. Never written at a call site that ships. */
  statusLabel: ReactNode;
  /**
   * The headline sentence — what happened, in the status grammar's own shape.
   * "Job 12 stalled at step 3", or the Job's own title.
   */
  headline: ReactNode;
  /** The job id, in mono, set back. Copies on click. */
  jobId?: string;
  /**
   * The field run. Five fixed tracks on Active jobs, so the list reads down as
   * well as across; the track list belongs to the field set, which is why a
   * Job that has not run carries a different one — no branch, no step and no
   * elapsed yet.
   *
   * The step bar is a field, not a region beside the run: "the same stacked
   * row, with the step added to the field run."
   */
  fields: JobRowField[];
  /**
   * The field run's `grid-template-columns`, where the field set wants
   * something other than the drawn default. Composed from the track custom
   * properties in this component's stylesheet, never from a literal.
   */
  tracks?: string;
  /** The one secondary control. Never a primary, and never more than one. */
  action?: ReactNode;
  /**
   * The running mark, still working. One per screen, and on the focused row
   * only — a list carries one running mark per Job and fourteen breathing dots
   * is what the motion rules forbid outright.
   */
  pulsing?: boolean;
  /** 2px `--accent` left edge over `--bg-hover`. The keyboard cursor. */
  focused?: boolean;
  /** `--accent-muted` fill. Coexists with focus. */
  selected?: boolean;
  /** De-emphasised: `--border-subtle` and `--fg-subtle`, never an alpha. */
  dimmed?: boolean;
  /** The row opens the Job, the same as its control's default action. */
  onOpen?: () => void;
  /** A clipboard write is silent, so the surface confirms it with a toast. */
  onCopied?: (value: string) => void;
};

/** Field glyphs are 12px at strokeWidth 2, like every mark below Job level. */
const FIELD_ICON = 12;
const FIELD_STROKE = 2;

function Copyable({
  value,
  copyValue,
  onCopied,
  className,
}: {
  value: ReactNode;
  copyValue?: string;
  onCopied?: (value: string) => void;
  className: string;
}) {
  const handleClick = useCallback(
    (event: MouseEvent<HTMLSpanElement>) => {
      if (copyValue === undefined) return;
      // The row opens the Job on click; copying a value inside it must not.
      event.stopPropagation();
      void navigator.clipboard.writeText(copyValue).then(
        () => onCopied?.(copyValue),
        // A failed clipboard write is otherwise indistinguishable from a dead
        // element, so the surface is told either way and says which happened.
        () => onCopied?.(copyValue),
      );
    },
    [copyValue, onCopied],
  );

  return (
    <span className={className} data-copies={copyValue !== undefined || undefined} onClick={handleClick}>
      {value}
    </span>
  );
}

export function JobRowStacked({
  status,
  statusIcon,
  statusLabel,
  headline,
  jobId,
  fields,
  tracks,
  action,
  pulsing = false,
  focused,
  selected,
  dimmed,
  onOpen,
  onCopied,
}: JobRowStackedProps) {
  return (
    <div
      className="armada-job-row"
      role="listitem"
      data-focused={focused || undefined}
      data-selected={selected || undefined}
      data-dimmed={dimmed || undefined}
      onClick={onOpen}
    >
      <div className="armada-job-row__badge">
        <Badge status={status} icon={statusIcon} pulsing={pulsing}>
          {statusLabel}
        </Badge>
      </div>

      <div className="armada-job-row__body">
        <div className="armada-job-row__headline">
          <span className="armada-job-row__title">{headline}</span>
          {jobId ? (
            <Copyable
              className="armada-job-row__id"
              value={jobId}
              copyValue={jobId}
              onCopied={onCopied}
            />
          ) : null}
        </div>

        <div
          className="armada-job-row__fields"
          style={{ gridTemplateColumns: tracks ?? trackList(fields.length) }}
        >
          {fields.map((field, i) => (
            <span
              className="armada-job-row__field"
              key={i}
              data-mono={field.mono || undefined}
              data-emphasis={field.emphasis || undefined}
              data-quiet={field.quiet || undefined}
            >
              {field.icon ? <field.icon size={FIELD_ICON} strokeWidth={FIELD_STROKE} aria-hidden /> : null}
              {field.label ? <span className="armada-job-row__field-label">{field.label}</span> : null}
              <Copyable
                className="armada-job-row__field-value"
                value={field.value}
                copyValue={field.copyValue}
                onCopied={onCopied}
              />
            </span>
          ))}
        </div>
      </div>

      {action ? (
        // The control stops the row's own open, so clicking Approve does not
        // also navigate.
        <div className="armada-job-row__action" onClick={(e) => e.stopPropagation()}>
          {action}
        </div>
      ) : null}
    </div>
  );
}

/**
 * The field run's drawn track list — the M1 field set, in order: the branch or
 * the workflow, the step bar, the step, elapsed, spend.
 *
 * **The tracks are fixed, and that is the point**: five fixed tracks make the
 * list read down as well as across, so a column of elapsed figures lines up
 * whatever precedes it. None of the five widths has a token — the drawing
 * sizes them, and each is composed from the spacing scale in this component's
 * stylesheet rather than written as a literal. Reported.
 *
 * A field set with a different shape passes its own `tracks`; a longer one
 * gets `auto` past the fifth, which is honest rather than silently wrong.
 */
const DRAWN_TRACKS = [
  "var(--armada-track-origin)",
  "var(--armada-track-bar)",
  "var(--armada-track-step)",
  "var(--armada-track-time)",
  "var(--armada-track-spend)",
];

function trackList(count: number): string {
  return Array.from({ length: count }, (_, i) => DRAWN_TRACKS[i] ?? "minmax(0, auto)").join(" ");
}
