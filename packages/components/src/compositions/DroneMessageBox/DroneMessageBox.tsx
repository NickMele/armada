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
      {/* Send sits inside the field's own frame, at its trailing foot, over
          the well rather than beside it — the owner's note of 17 Sep 2026.
          The room it occupies is reserved as the field's bottom padding
          (DroneMessageBox.css), so a typed line never runs underneath it. */}
      <div className="armada-drone-message-box__field">
        <Textarea
          aria-label="Message the drone"
          // Three rows, because a note to a drone is a sentence or two and one
          // row made every one of them scroll while being written.
          rows={3}
          value={value}
          onChange={(event) => onChange(event.target.value)}
          disabled={disabled}
          placeholder={placeholder}
        />
        {/* `sunken`: Send now rests on the field's own --bg-sunken well, and a
            secondary is filled one surface step from its ground (Button) — on
            `card` it would be the same fill as the field it sits in. */}
        <Button
          type="submit"
          variant="secondary"
          ground="sunken"
          size="sm"
          disabled={disabled || blank}
        >
          Send
        </Button>
      </div>
    </form>
  );
}
