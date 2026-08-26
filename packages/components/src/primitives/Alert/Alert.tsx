import type { ReactNode } from "react";

/**
 * A standing condition on a surface. Not a floating layer, so no shadow —
 * elevation is surface, and shadows are legal only on dialog, sheet, popover,
 * dropdown, tooltip and the palette.
 *
 * The contract's component mapping does not carry Alert. The two tones below
 * are read off the component sheet, which draws exactly two: a condition in
 * the escalation hue, and the neutral Doctor condition strip. Neither tone is
 * chosen — escalated takes the Job token, and neutral takes no hue at all.
 *
 * The glyph is the caller's, because Alert is not the thing that decides which
 * glyph a condition owns.
 */
export type AlertTone = "escalated" | "neutral";

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
};

export function Alert({ tone = "escalated", title, children, icon, action }: AlertProps) {
  return (
    <div
      className={
        tone === "neutral"
          ? "armada-alert armada-alert--neutral"
          : "armada-alert armada-alert--escalated"
      }
      role="status"
    >
      {icon ? <span className="armada-alert__glyph">{icon}</span> : null}
      <div className="armada-alert__copy">
        {title ? <span className="armada-alert__title">{title}</span> : null}
        <span className="armada-alert__body">{children}</span>
      </div>
      {action ? <span className="armada-alert__action">{action}</span> : null}
    </div>
  );
}
