import type { LucideIcon } from "lucide-react";
import type { CSSProperties, KeyboardEvent, MouseEvent, ReactNode } from "react";
import { createContext, useCallback, useContext } from "react";
import { Badge } from "../../primitives/Badge/Badge";

/**
 * Where this row sits in a roving list, supplied by the list that holds it.
 *
 * **A listbox is one tab stop, not one per row.** Tab reaches the list, the
 * arrows move within it, and that means exactly one option carries
 * `tabIndex=0` at a time — which the row cannot decide alone, because it does
 * not know its own position or which sibling the cursor is on.
 *
 * `null` is a row standing outside any list, which keeps its own tab stop.
 * Declared here rather than in `Active jobs list` because the list already
 * imports this file, and the reverse would be a cycle.
 */
export type Roving = { index: number; active: number };

export const RovingOption = createContext<Roving | null>(null);

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
 * rather than moving with the workflow's length. The rule is one pulse per
 * screen, on the most specific mark present — so on job detail the rail takes
 * it and this badge goes static, which is what `pulsing` is for.
 *
 * **Inside a roving list the pulse follows the cursor, not the status.** This
 * paragraph used to say the opposite — "a running row pulses whether or not
 * the cursor is on it" — and the Motion section of the design contract says
 * "on the focused row only", so the two disagreed and the contract wins. The
 * disagreement was unobservable while Fleet worked one Job: there was never a
 * second running row to pulse. There is now, and two marks breathing at
 * `--duration-pulse` is the thing that section forbids in its first sentence.
 * Hue still says which rows are running, on every one of them; the pulse says
 * *still working*, and that is only asked of the row being read.
 *
 * A row standing outside a roving list keeps `pulsing` as given — it is the
 * only row there is, so the cursor cannot single one out.
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
   * The field run. One track per field, shared down the list so it reads down
   * as well as across.
   *
   * The step bar is a field, not a region beside the run: "the same stacked
   * row, with the step added to the field run."
   */
  fields: JobRowField[];
  /**
   * The field run's `grid-template-columns`, where the field set wants
   * something other than the drawn default. Composed from the track custom
   * properties in this component's stylesheet, never from a literal.
   *
   * **The fallback's tracks, not the list's.** Inside a list the columns are
   * the list's and every row shares them, which is the alignment this used to
   * approximate per field set; this is what a row falls back to standing alone
   * or on an engine without `subgrid`.
   */
  tracks?: string;
  /** The one secondary control. Never a primary, and never more than one. */
  action?: ReactNode;
  /**
   * The running mark, still working. **Whether this Job is running**, which is
   * all a caller knows and all it is asked for.
   *
   * One per screen is the rule, and inside a roving list the row applies it
   * rather than the caller: the cursor's row takes the mark and every other
   * running row goes static. A caller cannot apply it, because it does not
   * know where the cursor is — the same reason `tabIndex` is not the caller's
   * either.
   */
  pulsing?: boolean;
  /** 2px `--accent` left edge over `--bg-hover`. The keyboard cursor. */
  focused?: boolean;
  /** `--accent-muted` fill. Coexists with focus. */
  selected?: boolean;
  /** De-emphasised: `--border-subtle` and `--fg-subtle`, never an alpha. */
  dimmed?: boolean;
  /**
   * The row opens the Job. Setting it makes the row a real control — a focus
   * stop that Enter and Space activate, announced as an option in the list it
   * selects from rather than as a listitem with a click handler bolted on.
   */
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
  // Enter and Space, because the row is a control when it opens something and
  // a control that only answers the mouse is unreachable. Space is stopped
  // from scrolling the list out from under the row it just activated.
  const handleKeyDown = useCallback(
    (event: KeyboardEvent<HTMLDivElement>) => {
      if (onOpen === undefined) return;
      if (event.key !== "Enter" && event.key !== " ") return;
      event.preventDefault();
      onOpen();
    },
    [onOpen],
  );

  const opens = onOpen !== undefined;
  // Outside a roving list every row is its own tab stop, which is right for a
  // row standing alone. Inside one, only the row the cursor is on is.
  const roving = useContext(RovingOption);
  const onCursor = roving === null || roving.index === roving.active;
  const tabIndex = !opens ? undefined : onCursor ? 0 : -1;
  // One animated mark per screen. `pulsing` says this Job is running; the
  // cursor says which running row is being read, and only that one breathes.
  const pulses = pulsing && onCursor;

  return (
    <div
      className="armada-job-row"
      // A row that selects a detail is an `option`, not a `listitem`: the list
      // it belongs to is a listbox, and `aria-selected` is what says which one
      // is open. A row that opens nothing stays a plain listitem.
      role={opens ? "option" : "listitem"}
      aria-selected={opens ? (selected ?? false) : undefined}
      tabIndex={tabIndex}
      data-job-id={jobId}
      data-focused={focused || undefined}
      data-selected={selected || undefined}
      data-dimmed={dimmed || undefined}
      onClick={onOpen}
      onKeyDown={opens ? handleKeyDown : undefined}
    >
      <div className="armada-job-row__badge">
        <Badge status={status} icon={statusIcon} pulsing={pulses}>
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

        {/* The track list is a custom property rather than
            `grid-template-columns` itself, because an inline
            `grid-template-columns` outranks every stylesheet — including the
            `subgrid` the list switches to when it can. */}
        <div
          className="armada-job-row__fields"
          style={{ "--armada-row-fallback-tracks": tracks ?? trackList(fields.length) } as CSSProperties}
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
 * The class the element holding the rows carries, so the field tracks are
 * declared once above every row instead of once inside each. `Active jobs
 * list` sets it; anything else that holds rows may.
 *
 * Exported rather than spelled twice: a magic string in two stylesheets is a
 * subgrid that silently stops being one when either side is renamed.
 */
export const JOB_ROW_LIST = "armada-job-row-list";

/**
 * The field run's drawn track list — the field set, in order: the branch or
 * the workflow, the step bar, the step, elapsed, spend, origin.
 *
 * **This is the fallback.** Inside a list the tracks come from the list and
 * these six widths are their floor; outside one, and on an engine without
 * `subgrid`, this is the whole answer. Fixed widths make the list read down as
 * well as across, so a column of elapsed figures lines up whatever precedes
 * it — but fixed is also why a row truncates on a wide window, which is what
 * the list's own tracks fix. None of the six has a token; each is composed
 * from the spacing scale in this component's stylesheet. Reported.
 *
 * Origin is sixth because the built row ran five — branch, step bar, step,
 * elapsed, spend — and the Board requires origin on every row besides. See
 * issue 218.
 *
 * A field set with a different shape passes its own `tracks`; a longer one
 * gets `auto` past the sixth, which is honest rather than silently wrong.
 */
const DRAWN_TRACKS = [
  "var(--armada-track-origin)",
  "var(--armada-track-bar)",
  "var(--armada-track-step)",
  "var(--armada-track-time)",
  "var(--armada-track-spend)",
  "var(--armada-track-provenance)",
];

function trackList(count: number): string {
  return Array.from({ length: count }, (_, i) => DRAWN_TRACKS[i] ?? "minmax(0, auto)").join(" ");
}
