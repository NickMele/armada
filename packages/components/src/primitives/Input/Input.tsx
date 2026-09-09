import { useId } from "react";
import type { ComponentPropsWithRef, ReactNode } from "react";

/**
 * A single-line text field, with its label and its invalid message.
 *
 * The three parts arrive together because the contract specifies them
 * together: an invalid input takes a `--status-completed-failed` border and
 * puts the message below it in `--text-xs`. Splitting the message out would
 * let a caller render the border with nothing saying why.
 *
 * Field labels never open with a Wh- word. The label is `Project location`.
 *
 * `ref` reaches the `input` itself rather than the wrapper, because what a
 * surface wants a reference to is the thing it puts the cursor in — the Job
 * Board's `/` focuses this field, and focusing the div around it would do
 * nothing. React 19 passes `ref` as an ordinary prop, so it rides `...rest`
 * to the element with the rest of them.
 */
export type InputProps = Omit<ComponentPropsWithRef<"input">, "size"> & {
  /** Sentence case, no Wh- opener. Omitted where a surface labels the field itself. */
  label?: string;
  /** The border goes to `--status-completed-failed` and `message` renders below. */
  invalid?: boolean;
  /** What is wrong, and what to do about it. Rendered only when `invalid`. */
  message?: string;
  /** Machine-derived content — a path, a branch, a command. Mono, one step smaller. */
  mono?: boolean;
  /**
   * Something drawn inside the field's own box, at its trailing edge — the key
   * that focuses it, a unit, a count.
   *
   * **Inside, because the field is what it belongs to.** A hint sitting beside
   * a search box is a second control on the line and takes the eye as one; the
   * same hint inside the box is a property of the field. What kept it outside
   * was ownership rather than design: the padding that makes room for it is
   * this component's to write, and a caller doing it from its own stylesheet
   * would be reaching into this one.
   *
   * The field gives up its own border when this is set and the box around the
   * pair takes it, so the two read as one control and the focus ring goes
   * round both. Nothing is reserved when it is absent.
   */
  trailing?: ReactNode;
};

export function Input({
  label,
  invalid = false,
  message,
  mono = false,
  trailing,
  id,
  ...rest
}: InputProps) {
  const generated = useId();
  const inputId = id ?? generated;
  const messageId = `${inputId}-message`;
  const showMessage = invalid && message !== undefined;

  return (
    <div className="armada-input-field">
      {label !== undefined && (
        <label className="armada-input-field__label" htmlFor={inputId}>
          {label}
        </label>
      )}
      {/* `display: contents` unless something is trailing, so a field with no
          adornment renders exactly the box it always did. */}
      <div className="armada-input-field__well" data-trailing={trailing !== undefined || undefined}>
        <input
          {...rest}
          id={inputId}
          className="armada-input"
          data-mono={mono || undefined}
          aria-invalid={invalid || undefined}
          aria-describedby={showMessage ? messageId : undefined}
        />
        {trailing === undefined ? null : (
          <span className="armada-input-field__trailing">{trailing}</span>
        )}
      </div>
      {showMessage && (
        <span className="armada-input-field__message" id={messageId}>
          {message}
        </span>
      )}
    </div>
  );
}
