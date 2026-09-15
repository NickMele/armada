import type { FormEvent } from "react";
import { Button } from "../../primitives/Button/Button";
import { Textarea } from "../../primitives/Textarea/Textarea";

/**
 * The message box fixed under an activity log — #1154. Sending it is a
 * redirect, same as the step header's own button and dialog; this is the
 * control for the moment a person is already reading the log and does not
 * want to leave it to reach one.
 *
 * **One component, two mounts.** The inline log and the Activity log sheet
 * each put this at their own foot, so the two cannot draw the availability
 * rule two different ways — `screens`' wiring reads `steeringOf` once and
 * hands both the same answer.
 *
 * **Disabled is not blank.** A field that goes grey with nothing said reads as
 * broken; `disabledReason` is the one line a caller gives for why a step with
 * no drone on it cannot take a message.
 */
export type DroneMessageBoxProps = {
  value: string;
  onChange: (value: string) => void;
  onSend: () => void;
  /** No drone is on this step to send to. */
  disabled?: boolean;
  /** Said in the field's place while `disabled` — why, in one line. */
  disabledReason?: string;
  /**
   * A redirect already out and unanswered, in `steering.ts`'s own words —
   * `waiting()`'s sentence. Shown above the field, which stays open: sending
   * again is legal and replaces what is outstanding, as it does today.
   */
  waiting?: string;
  placeholder?: string;
};

export function DroneMessageBox({
  value,
  onChange,
  onSend,
  disabled = false,
  disabledReason,
  waiting,
  placeholder = "Message the drone",
}: DroneMessageBoxProps) {
  const blank = value.trim() === "";

  function submit(event: FormEvent): void {
    event.preventDefault();
    if (!blank && !disabled) onSend();
  }

  return (
    <form className="armada-drone-message-box" onSubmit={submit}>
      {disabled
        ? disabledReason === undefined
          ? null
          : (
            <p className="armada-drone-message-box__said" role="note">
              {disabledReason}
            </p>
          )
        : waiting === undefined
          ? null
          : (
            <p className="armada-drone-message-box__said" role="status">
              {waiting}
            </p>
          )}
      <div className="armada-drone-message-box__row">
        <Textarea
          aria-label="Message the drone"
          rows={1}
          value={value}
          onChange={(event) => onChange(event.target.value)}
          disabled={disabled}
          placeholder={placeholder}
        />
        <Button type="submit" variant="secondary" size="sm" disabled={disabled || blank}>
          Send
        </Button>
      </div>
    </form>
  );
}
