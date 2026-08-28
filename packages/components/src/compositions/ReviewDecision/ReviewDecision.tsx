import type { ReactNode } from "react";
import { Button } from "../../primitives/Button/Button";
import { Separator } from "../../primitives/Separator/Separator";
import { Textarea } from "../../primitives/Textarea/Textarea";

/**
 * Review decision — the three answers to a Job waiting at a human gate, and the
 * note one of them carries.
 *
 * **The note is on this surface, never behind a control.**
 * `docs/practices/bridge.md`: reviewing a Drone's output and replying to it is
 * one continuous interaction, and a design that puts the reply in a separate
 * route, tab or modal from the diff recreates v1's problem inside Electron. So
 * the field sits where the diff is, already open, with nothing to press to
 * reach it.
 *
 * **Three acts, and two of them are recoverable.** That difference is the whole
 * job of this component's arrangement, and it is carried by position and by a
 * sentence rather than by a shade of red:
 *
 * | Act | Where it sits | What survives |
 * |---|---|---|
 * | Approve | the group, primary | the work is taken; on the last step Fleet commits and delivers |
 * | Request changes | the group, secondary | the drone, the worktree and the step — it goes back to work |
 * | Reject | below a rule, alone | **nothing. Terminal, and it ends the drone** |
 *
 * **Reject is not in the group and is not behind a caret.** A split button
 * would make it a variant of the act on its face, which is exactly the reading
 * it must not have: `crates/api/src/routes.rs` calls it a verdict on the work,
 * and the operations inventory calls it a hard stop. It sits below a rule, with
 * its own sentence, so what it costs is read before it is reached.
 *
 * **Approve does not confirm and reject does.** Approving is the ordinary path
 * — it is why the gate exists, and asking twice for the common case is a gate
 * in the wrong place. Rejecting ends two things, so the caller confirms it and
 * this only asks.
 *
 * **Request changes is refused with a blank note**, before the press, matching
 * the 422 Fleet gives it. A round trip to learn the field was empty is a
 * refusal a person reads as a failure.
 *
 * **No glyph on any of the three.** Primary and secondary are label-only by
 * contract, and a mark on the destructive one alone would make the difference
 * between the three a picture rather than a sentence — which is the reading
 * that lets a person press the terminal one thinking it is the loud version of
 * the mild one. The labels say what each does; the sentences say what survives.
 */
export type ReviewDecisionProps = {
  /** The reviewer's own words. Controlled — the caller holds the draft. */
  note: string;
  onNote: (note: string) => void;
  /** Take the work. Sent on the press. */
  onApprove: () => void;
  /** Send it back with the note. Refused while the note is blank. */
  onRequestChanges: () => void;
  /** Ask to reject. **The caller confirms**, because this ends two things. */
  onReject: () => void;
  /**
   * Every control off. A decision in flight, or nothing live to send it over —
   * the caller's sentence says which, since a disabled group with no reason is
   * a surface that looks broken.
   */
  disabled?: boolean;
  /** Why the controls are off, where they are. Never left to be guessed at. */
  disabledNote?: ReactNode;
  /** The label over the note field. Sentence case, no Wh- opener. */
  noteLabel?: string;
  /** What the two recoverable acts do, said once beneath them. */
  keptNote?: ReactNode;
  /** What rejecting costs. Its own sentence, beneath its own rule. */
  rejectNote?: ReactNode;
  approveLabel?: string;
  requestChangesLabel?: string;
  rejectLabel?: string;
};

export function ReviewDecision({
  note,
  onNote,
  onApprove,
  onRequestChanges,
  onReject,
  disabled = false,
  disabledNote,
  noteLabel = "What should change",
  keptNote = "Approving takes the work. Requesting changes sends this note to the drone as a turn — it keeps the worktree and the step, and comes back running.",
  rejectNote = "Rejecting is a verdict on the work and the job ends there. The drone is stopped and nothing resumes it. Its branch stays where the drone left it.",
  approveLabel = "Approve the work",
  requestChangesLabel = "Request changes",
  rejectLabel = "Reject the work",
}: ReviewDecisionProps) {
  const blank = note.trim() === "";

  return (
    <div className="armada-decision">
      {/* First, and always open. The reply is half of the loop this surface is,
          and a field behind a control is a second surface. */}
      <Textarea
        label={noteLabel}
        rows={4}
        value={note}
        disabled={disabled}
        onChange={(event) => onNote(event.target.value)}
      />

      <div className="armada-decision__kept">
        {/* The one accent fill on this surface. Approving is the act the gate
            exists for, and the distance from the red below is what keeps the
            two from reading as a pair. */}
        <Button variant="primary" disabled={disabled} onClick={onApprove}>
          {approveLabel}
        </Button>
        {/* Off while the note is blank, which is what Fleet would answer. */}
        <Button
          variant="secondary"
          disabled={disabled || blank}
          onClick={onRequestChanges}
        >
          {requestChangesLabel}
        </Button>
      </div>
      <p className="armada-decision__said">{keptNote}</p>

      {/* The rule is load-bearing, not decoration: it is what says the control
          under it is not another answer in the group above. */}
      <Separator decorative={false} className="armada-decision__rule" />

      <div className="armada-decision__terminal">
        {/* Outlined, because a solid red control reads as an error state rather
            than as an act. Alone, because it is the only one of the three that
            leaves nothing behind. */}
        <Button variant="destructive" disabled={disabled} onClick={onReject}>
          {rejectLabel}
        </Button>
        <p className="armada-decision__said" data-terminal>
          {rejectNote}
        </p>
      </div>

      {disabled && disabledNote !== undefined ? (
        <p className="armada-decision__said" role="note">
          {disabledNote}
        </p>
      ) : null}
    </div>
  );
}
