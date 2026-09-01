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
 * for each other. Three are closed here: one red, a leading edge and never a
 * box, and no tint on a data surface. See #228.
 *
 * **A boundary fallback may have no wire code, and the chip still draws.** The
 * treatment says the code is always shown because `ipc::WireError` guarantees
 * one; a renderer that threw is built from a caught exception, and
 * `apps/desktop/src/renderer/src/failures.ts` is right that Bridge must not
 * mint a code — an invented one is in no manifest and means nothing to the
 * lookup. So the chip carries the **region** instead: same solid fill, same
 * mono, same not-a-status claim, and honest about being a name rather than a
 * code. Where neither is supplied no chip draws, which is the remaining half
 * of #228 and is a call site's to fix, not this component's.
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
   * The `code` off the wire, where the failure came off the wire. It is what a
   * person reads back to someone else, so it is shown whenever it exists.
   */
  code?: string;
  /**
   * What failed, where there is no wire code — `Job board`, `Fleet
   * connection`. **A name, not a code**, and drawn in the code chip because
   * the chip's job is to say *error rather than status*, which is true of a
   * boundary fallback too.
   *
   * Ignored when `code` is given: a real code always outranks a region.
   */
  region?: string;
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
  region,
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
      {code === undefined && region === undefined ? null : (
        <div className="armada-failure__code">
          <ErrorCode kind="fault" code={code ?? (region as string)} />
        </div>
      )}

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
