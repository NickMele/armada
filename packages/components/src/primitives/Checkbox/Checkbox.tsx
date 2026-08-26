import { Check } from "lucide-react";
import type { InputHTMLAttributes, ReactNode } from "react";

/**
 * A checkbox and its label, as one hit target.
 *
 * Checkbox inherits the input's border and focus rules. Checked is an accent
 * fill carrying a `check` in `--fg-inverse` — the registry's bare checkmark,
 * which means yes in both the places it appears.
 */
export type CheckboxProps = Omit<InputHTMLAttributes<HTMLInputElement>, "type"> & {
  /** Sentence case, no Wh- opener. */
  children: ReactNode;
};

export function Checkbox({ children, ...rest }: CheckboxProps) {
  return (
    <label className="armada-checkbox">
      <input {...rest} type="checkbox" className="armada-checkbox__input" />
      <span className="armada-checkbox__box" aria-hidden="true">
        <Check size={12} strokeWidth={2} />
      </span>
      <span>{children}</span>
    </label>
  );
}
