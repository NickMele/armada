import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { useState } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";

import { ErrorCode } from "../../errors/ErrorCode/ErrorCode";
import { JobLogReference } from "../JobLogReference/JobLogReference";

/**
 * Failure notice — what broke, what to do, and the values that make a report
 * answerable.
 *
 * **One shape, never one message.** Three failures rendered through one shell
 * is the same discipline as six Job states rendered through one row shape; a
 * generic error screen is three failures given one *sentence*, which is the
 * thing this exists to stop. The caller supplies the sentence, the details and
 * the actions, and every one of the three differs in all three.
 *
 * **This is a fault, and it takes the error treatment.** It predated it, and
 * for a while drew a fault in the escalation hue on a 12% tint inside a box,
 * carrying no code — four disagreements with a treatment whose whole argument
 * is that an error and a failed Job are both red and must never be mistaken
 * for each other. All four are closed: one red, a leading edge and never a
 * box, no tint on a data surface, and the code below. See #228.
 *
 * **The code is required, and a boundary fallback has one.** The treatment
 * says the code is always shown, and the reason it can say *always* is that
 * the code is one of the two channels separating an error from a failed Job —
 * an optional prop here would have made the separation optional. Only one of
 * Bridge's five failures crosses the wire, so the other four mint their own in
 * the `bridge.` namespace; `codes.ts` beside `ErrorCode` carries why, and
 * `failures.ts` in `@armada/shell` carries the declarations. This drew the
 * **region** in the chip's place for a while, which kept the fill and lost the
 * meaning: a region names what stopped drawing, not what went wrong.
 *
 * **No glyph.** `triangle-alert` is reserved to Doctor and `octagon-alert` to
 * `stalled`, and the registry carries no mark for a Bridge failure. The code
 * and the sentence do the work rather than a glyph that means something else.
 */
export type FailureDetail = {
  /**
   * What the value is. Sentence case and no trailing colon, except where the
   * label is itself machine-derived — a `WireError`'s `fields` keys are the
   * wire's spelling and are not rewritten into prose here.
   */
  label: string;
  /** The value. Mono, because everything folded away here is machine-derived. */
  value: string;
};

export type FailureMachineValue = {
  /** 12px, strokeWidth 2, from the icon registry. `file` for a log. */
  icon?: LucideIcon;
  /** The accessible name for the glyph, since the value beside it is a path. */
  iconLabel?: string;
  value: string;
  /** What clicking the value puts on the clipboard. */
  copyValue?: string;
  /**
   * What the value names — "Fleet run" beside a run id. Mono and neutral, and
   * it says what the id identifies rather than implying it names this failure.
   */
  meta?: ReactNode;
  /** A rule above this row, where it starts a second group. */
  separated?: boolean;
};

export type FailureNoticeProps = {
  /** What broke, in one sentence. Never what threw, and never an apology. */
  headline: string;
  /**
   * The code. Off the wire where the failure crossed it, and minted by Bridge
   * in the `bridge.` namespace where it did not.
   *
   * **Required, because the treatment says always.** It is what a person reads
   * back to someone else, and it is one of the two channels that keep a
   * failing Armada apart from a failed Job. A caller with nothing to put here
   * is a failure with no name, which is the gap `codes.ts` exists to close
   * rather than a state this draws.
   */
  code: string;
  /** What to do. A failure with nothing to do is drawn as the dead end it is. */
  next: string;
  /** The stack, the payload, the component. Present, folded, never in the way. */
  details?: FailureDetail[];
  /** How the fold is named. "Details" unless the failure has a better word. */
  detailsLabel?: string;
  /** The log path, and a run id where one exists. Mono, copy on click, no glyph. */
  values?: FailureMachineValue[];
  /** What the machine values do not say. Stated, not implied. */
  note?: ReactNode;
  /** Reload at minimum, and the report that carries the details. */
  actions?: ReactNode;
  /** A clipboard write is silent, so the surface confirms it with a toast. */
  onCopied?: (value: string) => void;
};

/** Chrome glyphs are 16px at strokeWidth 2. The fold is chrome, not data. */
const FOLD_ICON = 16;
const FOLD_STROKE = 2;

export function FailureNotice({
  headline,
  code,
  next,
  details,
  detailsLabel = "Details",
  values,
  note,
  actions,
  onCopied,
}: FailureNoticeProps) {
  // `<details>` carries the keyboard and the accessible state; React is told
  // only so the caret matches, since the registry pairs two glyphs for
  // expand and collapse rather than rotating one.
  const [open, setOpen] = useState(false);
  const folded = details !== undefined && details.length > 0;
  const referenced = (values !== undefined && values.length > 0) || note !== undefined || actions !== undefined;

  return (
    <section className="armada-failure" role="alert">
      <div className="armada-failure__copy">
        <span className="armada-failure__headline">{headline}</span>
        <span className="armada-failure__next">{next}</span>
      </div>

      {/* The chip leads nothing and closes nothing — it sits under the copy,
          where the error treatment puts it, so the sentence is what is read
          first and the value a person quotes is what is read next. */}
      <div className="armada-failure__code">
        <ErrorCode kind="fault" code={code} />
      </div>

      {folded ? (
        <details
          className="armada-failure__details"
          onToggle={(event) => setOpen(event.currentTarget.open)}
        >
          <summary className="armada-failure__summary">
            {open ? (
              <ChevronDown size={FOLD_ICON} strokeWidth={FOLD_STROKE} aria-hidden />
            ) : (
              <ChevronRight size={FOLD_ICON} strokeWidth={FOLD_STROKE} aria-hidden />
            )}
            {detailsLabel}
          </summary>
          <dl className="armada-failure__list">
            {/* Keyed by position: two labels can repeat, since a `fields` key
                arriving off the wire is not guaranteed unique against the rows
                above it, and the list is rebuilt whole on every render. */}
            {details.map((detail, at) => (
              <div className="armada-failure__pair" key={at}>
                <dt className="armada-failure__label">{detail.label}</dt>
                <dd className="armada-failure__value">{detail.value}</dd>
              </div>
            ))}
          </dl>
        </details>
      ) : null}

      {referenced ? (
        <JobLogReference rows={values ?? []} actions={actions} onCopied={onCopied}>
          {note}
        </JobLogReference>
      ) : null}
    </section>
  );
}
