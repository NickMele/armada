import type { ReactNode } from "react";

/**
 * A standing condition on a surface. Not a floating layer, so no shadow —
 * elevation is surface, and shadows are legal only on dialog, sheet, popover,
 * dropdown, tooltip and the palette.
 *
 * Three tones. Two are read off the component sheet, which draws exactly two:
 * a condition in the escalation hue, and the neutral Doctor condition strip.
 * Neither is chosen — escalated takes the Job token, and neutral takes no hue
 * at all. The third, `caution`, is a contract addition: the one Alert tone
 * below Job level that takes hue, aliasing `--status-awaiting-review` as
 * `--notice-caution` per `docs/contracts/design-system.md`, Below Job level.
 * A heads-up about an act a person is about to take, not a Job state.
 *
 * The glyph is the caller's, because Alert is not the thing that decides which
 * glyph a condition owns. `caution` carries none by contract — `triangle-alert`
 * is Doctor's and the contract has no generic alarm glyph.
 */
export type AlertTone = "escalated" | "caution" | "neutral";

const TONE_CLASS: Record<AlertTone, string> = {
  escalated: "armada-alert--escalated",
  caution: "armada-alert--caution",
  neutral: "armada-alert--neutral",
};

export type AlertProps = {
  tone?: AlertTone;
  /** The headline sentence. Sentence case, and it names what happened. */
  title?: ReactNode;
  /** The facts needed to decide, on screen, without a click. */
  children: ReactNode;
  /** 16px, strokeWidth 2, from the icon registry. */
  icon?: ReactNode;
  /** One ghost control at most. A standing condition is not a decision queue. */
  action?: ReactNode;
  /**
   * Where the action sits. `block`, the default, centres it on the whole
   * condition — right for a strip of a line or two, and what every caller but
   * one draws.
   *
   * `title` puts it on the title's own line at the trailing edge, the way
   * `CardHeader` draws a card's, for an alert whose body is tall enough that
   * centred lands the control nowhere: the owner's note of 2026-09-17 on the
   * composer's repository ask was that it "just kind of sits in the middle".
   * It needs a `title` — without one there is no head for it to sit on, and
   * it falls back to the centred slot.
   */
  actionOn?: "block" | "title";
};

export function Alert({ tone = "escalated", title, children, icon, action, actionOn = "block" }: AlertProps) {
  // On the head, or in the trailing slot — never both, and never the head
  // when there is no title to draw a head from.
  const onHead = Boolean(action) && actionOn === "title" && Boolean(title);
  return (
    <div
      className={[
        "armada-alert",
        TONE_CLASS[tone],
        // A headline makes the copy a block, and the glyph belongs beside its
        // first line. Without one there is a single line to sit against, and
        // aligning to its top reads as a mistake rather than as alignment.
        title ? "armada-alert--stacked" : "armada-alert--single",
      ].join(" ")}
      role="status"
    >
      {icon ? <span className="armada-alert__glyph">{icon}</span> : null}
      <div className="armada-alert__copy">
        {!title ? null : onHead ? (
          <span className="armada-alert__head">
            <span className="armada-alert__title">{title}</span>
            <span className="armada-alert__action">{action}</span>
          </span>
        ) : (
          <span className="armada-alert__title">{title}</span>
        )}
        <span className="armada-alert__body">{children}</span>
      </div>
      {action && !onHead ? <span className="armada-alert__action">{action}</span> : null}
    </div>
  );
}
