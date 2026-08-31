import type { ReactNode } from "react";

import type { ErrorClass } from "../ErrorCode/ErrorCode";
import { ErrorCode } from "../ErrorCode/ErrorCode";

/**
 * An error, in one of the four places an error may appear.
 *
 * **One treatment, four placements, and the placement is picked by blast
 * radius rather than by severity.** A critical failure with a narrow radius
 * stays inline: approve-refused is red-serious and affects one row, so it
 * renders in that row and nowhere else. Escalating placement for severity is
 * the specific mistake this component exists to stop, which is why
 * `placement` has no default — there is nothing safe to fall back to.
 *
 * **Two classes, and only one is red.** `fault` is Armada unable to do the
 * thing. `degraded` is Armada unable to refresh what it is showing, and takes
 * no hue at all beyond an amber dot. Unreachable Fleet and dropped events are
 * degraded, not faults. The fixes are opposite — restarting Fleet is the wrong
 * move when the process is alive and only the stream stopped — so `kind` has
 * no default either.
 *
 * **One value, not a ladder.** Placement already carries blast radius, so
 * severity picks nothing but the edge. There is no second red, no `critical`
 * variant, and the copy at every placement is the same size.
 *
 * **Every placement names the failure and the act**, with one exception the
 * type enforces: a toast may carry no act, because it is the only placement
 * that reports something already over.
 *
 * **No glyph.** `triangle-alert` is Doctor's, `octagon-alert` is `stalled`'s,
 * and there is no generic alarm mark. The code and the sentence do the work.
 *
 * Where the thing sits is the caller's, and two classes are supplied for it:
 * `armada-error-toast-region` pins the bottom trailing corner inset
 * `--space-6` above the status bar, and `armada-error-surface-region` centres
 * a full-surface error in the region it replaces. Regions rather than props,
 * because a region holds several toasts and a component cannot own one.
 */
export type ErrorPlacement = "inline" | "toast" | "banner" | "surface";

export type ErrorField = {
  /** What the value is. Sentence case, no trailing colon. */
  label: string;
  /** The value. Mono, because the facts an error carries are machine-derived. */
  value: string;
};

type Common = {
  /** Fault or degraded, and never inferred from the message. */
  kind: ErrorClass;
  /**
   * The `code` off the wire. Required, because every error carries one and it
   * is what a person reads back to someone else.
   */
  code: string;
  /**
   * What failed, in one sentence. Event-first: the subject is the job or the
   * step, never the Drone. Never what threw, and never an apology.
   */
  message: ReactNode;
  /**
   * The facts needed to decide, on screen and without a click. The status
   * grammar's labelled field run, one step down from the sentence.
   */
  fields?: ErrorField[];
  /** Controls, where there is something to press. Never a second decision. */
  actions?: ReactNode;
};

/**
 * The act is required everywhere but a toast, and the union is what enforces
 * it. A rule written only in prose is one a call site can miss.
 */
export type ErrorNoticeProps =
  | (Common & {
      placement: "toast";
      /** A toast may report something already over and ask for nothing. */
      act?: ReactNode;
    })
  | (Common & {
      placement: "inline" | "banner" | "surface";
      /** What to do. A failure with nothing to do is drawn as the dead end it is. */
      act: ReactNode;
    });

export function ErrorNotice(props: ErrorNoticeProps) {
  const { kind, code, message, fields, actions, placement } = props;
  const act = props.act;

  return (
    <section
      className={`armada-error armada-error--${kind} armada-error--${placement}`}
      data-error-class={kind}
      data-placement={placement}
      // A fault has stopped something and interrupts; degraded means what is
      // on screen is stale, which is worth saying and not worth cutting a
      // screen reader off for.
      role={kind === "fault" ? "alert" : "status"}
    >
      <div className="armada-error__head">
        {kind === "degraded" ? <span className="armada-error__dot" aria-hidden="true" /> : null}
        <span className="armada-error__message">{message}</span>
      </div>

      {act !== undefined ? <span className="armada-error__act">{act}</span> : null}

      {/* The code leads the run of machine-derived facts rather than sitting
          beside the sentence. Trailing the headline it competes with the copy
          for the line, and at the toast's measure a long code takes half of it
          and reflows the sentence into a column — the exact failure this app
          was built to escape. Here it wraps with everything else, and it is
          with the values it is quoted alongside. */}
      <div className="armada-error__facts">
        <ErrorCode kind={kind} code={code} />
        {fields !== undefined && fields.length > 0 ? (
          <dl className="armada-error__fields">
            {/* Keyed by position: a `fields` key arriving off the wire is not
                guaranteed unique against the rows beside it, and the run is
                rebuilt whole on every render. */}
            {fields.map((field, at) => (
              <div className="armada-error__field" key={at}>
                <dt className="armada-error__label">{field.label}</dt>
                <dd className="armada-error__value">{field.value}</dd>
              </div>
            ))}
          </dl>
        ) : null}
      </div>

      {actions !== undefined ? <div className="armada-error__actions">{actions}</div> : null}
    </section>
  );
}
