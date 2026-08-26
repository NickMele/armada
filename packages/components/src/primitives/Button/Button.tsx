import type { ButtonHTMLAttributes, ReactNode } from "react";

/**
 * The four button variants of the design system contract.
 *
 * Emphasis comes from fill, not size. A primary is `--accent` fill at the
 * normal control height — never a scaled-up CTA — and there is one per view.
 * A list row never takes one: fourteen rows offering a decision would be
 * fourteen accent blocks, so a row carries a secondary and urgency is read
 * from the badge and the ordering.
 *
 * Primary and secondary are label-only. Icons belong on ghost row actions,
 * in confirmation dialogs and in toolbars.
 */
export type ButtonVariant = "primary" | "secondary" | "ghost" | "destructive";

export type ButtonProps = ButtonHTMLAttributes<HTMLButtonElement> & {
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
  children?: ReactNode;
};

export function Button({
  variant = "secondary",
  size = "default",
  ground = "card",
  iconOnly = false,
  type = "button",
  children,
  ...rest
}: ButtonProps) {
  return (
    <button
      {...rest}
      type={type}
      className="armada-button"
      data-variant={variant}
      data-size={size}
      data-ground={ground}
      data-icon-only={iconOnly || undefined}
    >
      {children}
    </button>
  );
}
