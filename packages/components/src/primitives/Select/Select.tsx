import { useId } from "react";
import type { ReactNode, SelectHTMLAttributes } from "react";
import { ChevronDown } from "lucide-react";

/**
 * A one-of-many field. Select inherits the input's border, focus and height
 * rules, so the two line up in a column of fields.
 *
 * The indicator is `chevron-down` rather than the platform's own. A native
 * select draws its arrow at a fixed inset from the border box, which no amount
 * of padding moves — so the gap between the glyph and the edge is whatever the
 * engine decided, and on this one it is too tight.
 *
 * The registry reserves `chevron-down` to disclosure and forbids it as a sort
 * or direction indicator, as an ornament on a label, and as an expander on a
 * row of data. A list of options opening in chrome is none of those; it is
 * disclosure, which is the reservation rather than an exception to it.
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
      <span className="armada-select-shell">
      <select
        {...rest}
        id={selectId}
        className="armada-select"
        aria-invalid={invalid || undefined}
        aria-describedby={showMessage ? messageId : undefined}
      >
        {children}
      </select>
        <ChevronDown className="armada-select__caret" size={16} strokeWidth={2} aria-hidden />
      </span>
      {showMessage && (
        <span className="armada-select-field__message" id={messageId}>
          {message}
        </span>
      )}
    </div>
  );
}
