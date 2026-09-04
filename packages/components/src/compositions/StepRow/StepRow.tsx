import { ChevronDown, ChevronRight, Lock } from "lucide-react";
import type { ReactNode } from "react";
import { StepActivityMark, type StepActivity } from "../StepActivityMark/StepActivityMark";

/**
 * One row of the run — a step, its mark, its elapsed figure, and the short
 * facts the chevron opens beneath it.
 *
 * **Four columns, and the widths are the drawing's**: a chevron, a mark, the
 * name, and the duration. The name is the only one that flexes, so a long step
 * name clips and nothing else on the row moves — which is what stops the
 * duration column from flip-flopping between renders, the complaint the v1
 * failure log recorded nine times.
 *
 * **The chevron opens the facts; the row selects the step.** Two controls, on
 * purpose: the tree holds the short facts a step produced and the panel beside
 * it holds anything that is a sentence. A tree that opened into prose is the
 * panel, badly.
 *
 * **Waiting, stopped and failed must never look alike.** They are three kinds
 * of stopped — waiting on you is the workflow working, stopped is a Drone that
 * tried and cannot get further, failed is over. Stopped and failed carry a row
 * surface as well as a mark, because a glyph only holds while its row is
 * selected and the row that ended the run has to stay findable while its
 * refusals are read beside it. Waiting carries neither: a tint would make the
 * workflow working look like the workflow failing.
 *
 * **A fact is a value.** `FactChip` and `PathChip` are what a caller puts in
 * one, and this row draws the name beside them and nothing else — a row that
 * formatted its own values would be the second place a chip is drawn.
 *
 * **The duration slot is always drawn.** A step that has not run shows `—`
 * rather than a blank, because a blank in a column of figures reads as a
 * value that failed to load.
 */

/**
 * One short fact beneath a step: what it produced, what it cleared, how many
 * attempts it took, what its Checks and its Judge came to.
 *
 * **Never a sentence.** Anything that reads as prose belongs in the panel,
 * where there is room for it — that division is the whole reason the tree and
 * the panel are two regions rather than one.
 */
export type StepRowFact = {
  /**
   * What the fact is, in sans: `Produced`, `Cleared`, `Attempt 2`, `Checks`,
   * `Judge`, `Held`. A name, so it reads as one.
   */
  label: ReactNode;
  /**
   * The value — one `FactChip`, one `PathChip`, or several of either. Passed
   * whole rather than as data, because the chips are components with their own
   * rules and re-deriving them here is how two drawings of one chip start.
   */
  value?: ReactNode;
  /**
   * That attempt's own Checks, Judge and Verdict, indented beneath it. Only
   * an attempt fact carries these, on a step worked more than once — see
   * `RunTreeFact.children`, which this is drawn from.
   */
  children?: StepRowFact[];
};

/** The em dash a step that has not run shows in place of a duration. */
export const NO_DURATION = "—";

export type StepRowProps = {
  /**
   * The step's name, in sans. Nouns naming the artifact — Reproduction, Root
   * cause, Fix, Regression check. Sans names work, mono names machinery.
   */
  label: ReactNode;
  /** Whether `label` is a `step_id` rather than a name, so it renders in mono. */
  labelIsAnIdentifier?: boolean;
  activity: StepActivity;
  /**
   * The activity in words — `waiting on you`, `retries spent`. **The mark's
   * accessible name, and nothing visible**: hue and silhouette are the two
   * visible channels and this is the third, because a mark whose only label is
   * its colour says nothing to a screen reader.
   */
  status?: string;
  /**
   * How long the step took, or has been going. Mono, measured, a figure and
   * never a bar — a filled bar reads as progress and a step has no
   * percentage. Absent draws `—`.
   */
  elapsed?: ReactNode;
  /**
   * The step's 1-based position, where the caller wants a number in the mark's
   * slot instead of a silhouette. **Job detail does not pass one**: the drawing
   * gives an unreached step a hollow ring, and a list number in a tree of six
   * says only that the tree has six rows.
   */
  ordinal?: number;
  /** Whether this is the step the panel is showing. One per tree. */
  selected?: boolean;
  /**
   * The drawn hover state, held open. **For the story and nothing else** — a
   * pseudo-class cannot be photographed, and a hover treatment nobody can look
   * at is one that drifts unseen.
   */
  hovered?: boolean;
  /** Whether the facts are open. The tree above holds this, not the row. */
  open?: boolean;
  /** Open or close the facts. Absent draws no chevron — nothing to open. */
  onToggle?: () => void;
  /** Select the step. Absent draws the name as a label rather than a control. */
  onSelect?: () => void;
  /**
   * Whether the step is a hard prerequisite of the workflow definition. A
   * `lock` at the name's trailing edge in `--fg-muted`, label only, with no
   * action behind it — the way past a locked step is Pilot.
   */
  locked?: boolean;
  /** What the lock says on hover and to a screen reader. */
  lockedLabel?: string;
  /** The short facts the chevron opens. Empty is a step that produced none. */
  facts?: StepRowFact[];
  /** What a step with no facts says instead of leaving a blank. */
  factsAbsent?: ReactNode;
  /**
   * The running mark, still working. One per screen, on the most specific mark
   * present and on the thing being read.
   */
  pulsing?: boolean;
  /**
   * What the facts region is called, for `aria-controls`. Unique per row, and
   * supplied by the tree, which is the thing that knows it has more than one.
   */
  factsId?: string;
};

/** Chrome glyphs on this row are 12px at strokeWidth 2, as the marks are. */
const GLYPH = 12;
const STROKE = 2;

export function StepRow({
  label,
  labelIsAnIdentifier,
  activity,
  status,
  elapsed,
  ordinal,
  selected,
  hovered,
  open = false,
  onToggle,
  onSelect,
  locked,
  lockedLabel,
  facts = [],
  factsAbsent,
  pulsing = false,
  factsId,
}: StepRowProps) {
  const lockSays = lockedLabel ?? "Cannot be skipped, even on retry";
  return (
    <div className="armada-srow-group">
      <div
        className="armada-srow"
        data-activity={activity}
        data-sel={selected || undefined}
        data-hover={hovered || undefined}
      >
        {/* The chevron and the name are two controls, not one. A button inside
            a button is not a thing the DOM has, and the two do different work.
            A step with nothing to open draws the column and leaves it empty,
            so the marks below it stay in one line. */}
        {onToggle === undefined ? (
          <span className="armada-srow__chevron" aria-hidden />
        ) : (
          <button
            type="button"
            className="armada-srow__chevron"
            aria-expanded={open}
            aria-controls={factsId}
            aria-label={open ? "Close this step's facts" : "Open this step's facts"}
            onClick={onToggle}
          >
            {open ? (
              <ChevronDown size={GLYPH} strokeWidth={STROKE} aria-hidden />
            ) : (
              <ChevronRight size={GLYPH} strokeWidth={STROKE} aria-hidden />
            )}
          </button>
        )}

        <StepActivityMark
          activity={activity}
          label={status ?? activity}
          ordinal={ordinal}
          pulsing={pulsing}
        />

        {onSelect === undefined ? (
          <span className="armada-srow__name" data-identifier={labelIsAnIdentifier || undefined}>
            {label}
            {locked ? <StepRowLock says={lockSays} /> : null}
          </span>
        ) : (
          <button
            type="button"
            className="armada-srow__name"
            data-identifier={labelIsAnIdentifier || undefined}
            aria-current={selected ? "true" : undefined}
            onClick={onSelect}
          >
            {label}
            {locked ? <StepRowLock says={lockSays} /> : null}
          </button>
        )}

        <span className="armada-srow__dur">{elapsed ?? NO_DURATION}</span>
      </div>

      {/* Kept in the document while closed so the chevron's `aria-controls`
          names something. Hidden, not unmounted — a reference to an element
          that is not there is worse than no reference. */}
      <div className="armada-srow__facts" id={factsId} hidden={!open}>
        {facts.length === 0 ? (
          <p className="armada-srow__absent">
            {factsAbsent ?? "Nothing was recorded against this step."}
          </p>
        ) : (
          facts.map((fact, at) => (
            <div className="armada-srow__fact" key={at}>
              <span className="armada-srow__fact-label">{fact.label}</span>
              {fact.value}
              {fact.children === undefined || fact.children.length === 0 ? null : (
                <div className="armada-srow__fact-children">
                  {fact.children.map((child, c) => (
                    <div className="armada-srow__fact" key={c}>
                      <span className="armada-srow__fact-label">{child.label}</span>
                      {child.value}
                    </div>
                  ))}
                </div>
              )}
            </div>
          ))
        )}
      </div>
    </div>
  );
}

/**
 * The lock, drawn inside the name so it travels with it. Label only: a hard
 * prerequisite is a property of the workflow definition rather than of this
 * run, so it takes the quietest treatment that survives being repeated three
 * times on one tree.
 */
function StepRowLock({ says }: { says: string }) {
  return (
    <span className="armada-srow__lock" title={says}>
      <Lock size={GLYPH} strokeWidth={STROKE} aria-hidden />
      <span className="armada-srow__sr">{says}</span>
    </span>
  );
}
