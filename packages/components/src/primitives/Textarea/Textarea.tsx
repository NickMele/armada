import { useId } from "react";
import type { TextareaHTMLAttributes } from "react";

/**
 * A multi-line text field, with its label and its invalid message.
 *
 * `textarea` is sanctioned by hard rule two because a Job's brief is prose a
 * person writes at length, and a single-line input for it is a control that
 * fights its content. The contract specifies no `Textarea` of its own, so
 * surface, edge, radius, type, focus, invalid and disabled all come from
 * `### Input` — the two line up in a column of fields, and a control that
 * behaves differently from an input for no reason is two controls to learn.
 *
 * Three things `Input` does not answer, decided here:
 *
 * - **Height is a row count, not a height.** `rows` sets it, defaulting to
 *   three. The mockup draws an 88px well and no token names that number; a row
 *   count is the same well spelled in the type scale, which is where a
 *   text field's height comes from anyway.
 * - **It does not resize.** A drag handle is chrome, and this is an instrument
 *   panel. A caller that needs a taller well passes `rows`.
 * - **It does not count characters.** No brief length limit is stated
 *   anywhere, and a counter would be the first place one was invented.
 *
 * Field labels never open with a Wh- word. The label is `Brief`.
 */
export type TextareaProps = TextareaHTMLAttributes<HTMLTextAreaElement> & {
  /** Sentence case, no Wh- opener. Omitted where a surface labels the field itself. */
  label?: string;
  /** The border goes to `--status-completed-failed` and `message` renders below. */
  invalid?: boolean;
  /** What is wrong, and what to do about it. Rendered only when `invalid`. */
  message?: string;
};

export function Textarea({
  label,
  invalid = false,
  message,
  rows = 3,
  id,
  ...rest
}: TextareaProps) {
  const generated = useId();
  const textareaId = id ?? generated;
  const messageId = `${textareaId}-message`;
  const showMessage = invalid && message !== undefined;

  return (
    <div className="armada-textarea-field">
      {label !== undefined && (
        <label className="armada-textarea-field__label" htmlFor={textareaId}>
          {label}
        </label>
      )}
      <textarea
        {...rest}
        rows={rows}
        id={textareaId}
        className="armada-textarea"
        aria-invalid={invalid || undefined}
        aria-describedby={showMessage ? messageId : undefined}
      />
      {showMessage && (
        <span className="armada-textarea-field__message" id={messageId}>
          {message}
        </span>
      )}
    </div>
  );
}
