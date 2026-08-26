import type { InputHTMLAttributes, ReactNode } from "react";

/**
 * A setting that is on or off, with its label on the left and the track on
 * the right.
 *
 * Switch inherits the input's border, focus and height rules, and takes
 * `--accent` when on.
 *
 * The optional description states what the setting does when on and what
 * happens when off. It is not help text for the label — a label that needs
 * explaining is the wrong label.
 */
export type SwitchProps = Omit<InputHTMLAttributes<HTMLInputElement>, "type"> & {
  /** Sentence case, no Wh- opener. */
  children: ReactNode;
  /** What happens when on, and what happens when off. */
  description?: ReactNode;
};

export function Switch({ children, description, ...rest }: SwitchProps) {
  return (
    <label className="armada-switch" data-described={description !== undefined || undefined}>
      <span className="armada-switch__text">
        <span>{children}</span>
        {description !== undefined && (
          <span className="armada-switch__description">{description}</span>
        )}
      </span>
      <input {...rest} type="checkbox" role="switch" className="armada-switch__input" />
      <span className="armada-switch__track" aria-hidden="true">
        <span className="armada-switch__thumb" />
      </span>
    </label>
  );
}
