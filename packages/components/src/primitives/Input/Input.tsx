import { useId } from "react";
import type { InputHTMLAttributes } from "react";

/**
 * A single-line text field, with its label and its invalid message.
 *
 * The three parts arrive together because the contract specifies them
 * together: an invalid input takes a `--status-completed-failed` border and
 * puts the message below it in `--text-xs`. Splitting the message out would
 * let a caller render the border with nothing saying why.
 *
 * Field labels never open with a Wh- word. The label is `Project location`.
 */
export type InputProps = Omit<InputHTMLAttributes<HTMLInputElement>, "size"> & {
  /** Sentence case, no Wh- opener. Omitted where a surface labels the field itself. */
  label?: string;
  /** The border goes to `--status-completed-failed` and `message` renders below. */
  invalid?: boolean;
  /** What is wrong, and what to do about it. Rendered only when `invalid`. */
  message?: string;
  /** Machine-derived content — a path, a branch, a command. Mono, one step smaller. */
  mono?: boolean;
};

export function Input({ label, invalid = false, message, mono = false, id, ...rest }: InputProps) {
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
      <input
        {...rest}
        id={inputId}
        className="armada-input"
        data-mono={mono || undefined}
        aria-invalid={invalid || undefined}
        aria-describedby={showMessage ? messageId : undefined}
      />
      {showMessage && (
        <span className="armada-input-field__message" id={messageId}>
          {message}
        </span>
      )}
    </div>
  );
}
