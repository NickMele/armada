import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { useState } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";

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
 * The escalation hue is not chosen per failure. There is one tone because a
 * failure is one standing condition, and picking a hue per kind would be
 * writing a status by hand for something that is not a Job.
 *
 * **No glyph.** `triangle-alert` is reserved to Doctor and `octagon-alert` to
 * `stalled`, and the registry carries no mark for a Bridge failure. The
 * headline does the work rather than a glyph that means something else.
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
