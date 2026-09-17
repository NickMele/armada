import { useEffect, useState, type ButtonHTMLAttributes, type ReactNode, type Ref } from "react";
import { useAnswerTap } from "../../haptics";

/**
 * The five button variants of the design system contract. `tonal` is chrome,
 * carried over from `SplitButton`'s own `tonal` for the title row's Dispatch
 * (#1156).
 *
 * Emphasis comes from fill, not size. A primary is `--accent` fill at the
 * normal control height — never a scaled-up CTA — and there is one per view.
 * A list row never takes one: fourteen rows offering a decision would be
 * fourteen accent blocks, so a row carries a secondary and urgency is read
 * from the badge and the ordering.
 *
 * Primary and secondary are label-only. Icons belong on ghost row actions,
 * in confirmation dialogs, in toolbars, and on `tonal`.
 */
export type ButtonVariant = "primary" | "secondary" | "ghost" | "destructive" | "tonal";

/**
 * What Fleet said to the press that was pending. The bottom edge's line takes
 * the answer's colour — it fills the edge on `accepted`, retracts on `refused` —
 * holds for `--duration-answer`, and rests. The line is decoration; the words
 * of a refusal are the caller's to draw.
 */
export type ButtonAnswer = "accepted" | "refused";

export type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
  /**
   * The element, for a caller that has to move focus to it — a sheet lands
   * focus on its close. React 19 passes `ref` to a function component as an
   * ordinary prop, so it reaches the element through the spread below and only
   * the type needed widening.
   */
  ref?: Ref<HTMLButtonElement>;
  variant?: ButtonVariant;
  /** `sm` inside table rows. Every button in one group takes the same size. */
  size?: "default" | "sm";
  /**
   * The surface the button sits on, which decides a secondary's fill: a
   * secondary is filled one surface step from its ground. `card` gives
   * `--bg-sunken`; `sunken` — a sunken well or an overlay row — gives
   * `--bg-raised`. Ignored by every other variant.
   */
  ground?: "card" | "sunken";
  /**
   * A ghost row action carrying a glyph and no label. `aria-label` is
   * required by the caller, since the glyph is the whole content.
   */
  iconOnly?: boolean;
  /**
   * This is the control that was pressed, and Fleet has not answered. It
   * sweeps a bar, stays focusable, and refuses a second press. The caller
   * disables the rest of the group and words the label as the act underway.
   * #1117.
   */
  pending?: boolean;
  /**
   * Fleet's answer to the press that was `pending`, set as `pending` clears.
   * Pending wins while both are set. The line plays once each time the answer
   * is newly set, so a caller clears it when the next press goes out rather
   * than on a timer — the hold is `--duration-answer`, the stylesheet's clock.
   *
   * Newly set on a control that was `pending`, it also plays the answer's
   * trackpad pattern through `HapticsProvider`, once.
   */
  answer?: ButtonAnswer;
  children?: ReactNode;
};

export function Button({
  variant = "secondary",
  size = "default",
  ground = "card",
  iconOnly = false,
  pending = false,
  answer,
  type = "button",
  disabled,
  onClick,
  children,
  ...rest
}: ButtonProps) {
  // The line says it to the eyes, and the trackpad to the finger that pressed.
  useAnswerTap(pending, answer);
  return (
    <button
      {...rest}
      type={type}
      className="armada-button"
      data-variant={variant}
      data-size={size}
      data-ground={ground}
      data-icon-only={iconOnly || undefined}
      data-pending={pending || undefined}
      data-answer={pending ? undefined : answer}
      // Not `disabled`: a disabled button drops focus and is skipped by a
      // screen reader, and this one is the thing being waited on.
      disabled={pending ? undefined : disabled}
      aria-disabled={pending || undefined}
      aria-busy={pending || undefined}
      onClick={pending ? undefined : onClick}
    >
      {children}
    </button>
  );
}

/** How long a press waits before the group says Fleet is still on it. */
export const STILL_WAITING_AFTER_MS = 5000;

/** The one sentence for a press Fleet has not answered after that long. */
export const STILL_WAITING = "Still waiting on Fleet. Pressing again won't send it twice.";

/** True once `pending` has held for `after` milliseconds; false the moment it clears. */
export function useStillWaiting(pending: boolean, after = STILL_WAITING_AFTER_MS): boolean {
  const [long, setLong] = useState(false);
  useEffect(() => {
    setLong(false);
    if (!pending) return;
    const timer = setTimeout(() => setLong(true), after);
    return () => clearTimeout(timer);
  }, [pending, after]);
  return pending && long;
}
