import { useId } from "react";
import type { ReactNode, SelectHTMLAttributes } from "react";

/**
 * A one-of-many field. Select inherits the input's border, focus and height
 * rules, so the two line up in a column of fields.
 *
 * It renders a native `select` and lets the platform draw its own indicator.
 * The icon registry reserves `chevron-down` to disclosure and forbids it as a
 * direction indicator, and no registered glyph means "this list opens" — so
 * borrowing one would be inventing an assignment the icon contract has not
 * made.
 */
export type SelectProps = SelectHTMLAttributes<HTMLSelectElement> & {
  /** Sentence case, no Wh- opener. */
  label?: string;
  invalid?: boolean;
  message?: string;
  children?: ReactNode;
};

export function Select({ label, invalid = false, message, id, children, ...rest }: SelectProps) {
  const generated = useId();
  const selectId = id ?? generated;
  const messageId = `${selectId}-message`;
  const showMessage = invalid && message !== undefined;

  return (
    <div className="armada-select-field">
      {label !== undefined && (
        <label className="armada-select-field__label" htmlFor={selectId}>
          {label}
        </label>
      )}
      <select
        {...rest}
        id={selectId}
        className="armada-select"
        aria-invalid={invalid || undefined}
        aria-describedby={showMessage ? messageId : undefined}
      >
        {children}
      </select>
      {showMessage && (
        <span className="armada-select-field__message" id={messageId}>
          {message}
        </span>
      )}
    </div>
  );
}
