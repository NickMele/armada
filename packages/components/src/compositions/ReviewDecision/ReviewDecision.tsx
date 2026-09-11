import type { ReactNode } from "react";
import { Button } from "../../primitives/Button/Button";
import { Separator } from "../../primitives/Separator/Separator";
import { Textarea } from "../../primitives/Textarea/Textarea";
import { Tooltip } from "../../primitives/Tooltip/Tooltip";

/**
 * Review decision — the answers to a Job waiting at a human gate, and the note
 * one of them carries.
 *
 * **The note is on this surface, never behind a control.**
 * `docs/practices/bridge.md`: reviewing a Drone's output and replying to it is
 * one continuous interaction, and a design that puts the reply in a separate
 * route, tab or modal from the diff recreates v1's problem inside Electron. So
 * the field sits where the diff is, already open, with nothing to press to
 * reach it.
 *
 * **Four acts, and three of them are recoverable.** That difference is the
 * whole job of this component's arrangement, and it is carried by position and
 * by a sentence rather than by a shade of red:
 *
 * | Act | Where it sits | What survives |
 * |---|---|---|
 * | Merge | the group, primary, **only where there is a pull request** | the work lands, and Fleet runs the repository's after-merge checks |
 * | Approve | the group | the work is taken and the pull request is left open |
 * | Request changes | the group, secondary | the drone, the worktree and the step — it goes back to work |
 * | Reject | below a rule, alone | **nothing. Terminal, and it ends the drone** |
 *
 * **Merge takes the primary fill from Approve when it is offered, and that is
 * the point of it.** A job holding an open pull request has one ordinary
 * ending, and it is not "record this done and leave the branch on the forge" —
 * that is the state the button exists to stop. Where there is no pull request
 * to merge the prop is absent, the control is not drawn, and Approve is the
 * primary act again.
 *
 * **Reject is not in the group and is not behind a caret.** A split button
 * would make it a variant of the act on its face, which is exactly the reading
 * it must not have: `crates/api/src/routes.rs` calls it a verdict on the work,
 * and the operations inventory calls it a hard stop. It sits below a rule, with
 * its own sentence, so what it costs is read before it is reached.
 *
 * **Approve does not confirm; merge and reject do.** Approving is the ordinary
 * path — it is why the gate exists, and asking twice for the common case is a
 * gate in the wrong place. Rejecting ends two things. Merging is the one act
 * here that writes into a repository Fleet did not make, and nothing in Bridge
 * takes it back. Both are the caller's to confirm, and this only asks.
 *
 * **Request changes is refused with a blank note**, before the press, matching
 * the 422 Fleet gives it. A round trip to learn the field was empty is a
 * refusal a person reads as a failure.
 *
 * **No glyph on any of them.** Primary and secondary are label-only by
 * contract, and a mark on the destructive one alone would make the difference
 * between them a picture rather than a sentence — which is the reading that
 * lets a person press the terminal one thinking it is the loud version of the
 * mild one. The labels say what each does; the sentences say what survives.
 */
export type ReviewDecisionProps = {
  /** The reviewer's own words. Controlled — the caller holds the draft. */
  note: string;
  onNote: (note: string) => void;
  /**
   * Ask to merge the pull request and take the work. **The caller confirms**,
   * because this writes into a repository Fleet did not make and Bridge cannot
   * undo it — the same shape as `onReject`, and for the other of the two
   * reasons an answer here is worth a second press.
   *
   * **Absent is a job with no pull request to merge** — a workflow that
   * declares no delivering step opened none, and so did one whose push failed.
   * Presence is the whole of what decides this control, because a merge Fleet
   * would refuse must not be a button a person can reach.
   */
  onMerge?: () => void;
  /** Take the work, leaving the pull request where it is. Sent on the press. */
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
  /** What approving does, on hover over its control. */
  approveNote?: ReactNode;
  /** What requesting changes does, on hover over its control. */
  requestChangesNote?: ReactNode;
  /** What merging does, on hover over its control. */
  mergeNote?: ReactNode;
  /** What rejecting costs, on hover over its control. */
  rejectNote?: ReactNode;
  mergeLabel?: string;
  approveLabel?: string;
  requestChangesLabel?: string;
  rejectLabel?: string;
};

export function ReviewDecision({
  note,
  onNote,
  onMerge,
  onApprove,
  onRequestChanges,
  onReject,
  disabled = false,
  disabledNote,
  noteLabel = "What should change",
  approveNote = "Takes the work as the drone left it.",
  requestChangesNote = "Sends this note to the drone as a turn. It keeps the worktree and the step, and comes back running.",
  mergeNote = "Merges the pull request on its code host, then takes the work. Armada runs the repository's after-merge checks against what landed; merging it there yourself skips them.",
  rejectNote = "A verdict on the work, and the job ends there. The drone is stopped and nothing resumes it. Its branch stays where the drone left it.",
  mergeLabel = "Merge and take the work",
  approveLabel = "Approve the work",
  requestChangesLabel = "Request changes",
  rejectLabel = "Reject the work",
}: ReviewDecisionProps) {
  const blank = note.trim() === "";
  // Presence and never a flag: the caller has the pull request or it has not,
  // and a boolean beside a handler would let a surface offer an act with
  // nothing behind it.
  const merging = onMerge !== undefined;

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
        {/* The one accent fill on this surface, and it moves. A job with a pull
            request open has one ordinary ending and it is this one; a job with
            none never draws this control at all. */}
        {merging ? (
          <Tooltip label={mergeNote}>
            <Button variant="primary" disabled={disabled} onClick={onMerge}>
              {mergeLabel}
            </Button>
          </Tooltip>
        ) : null}
        <Tooltip label={approveNote}>
          <Button
            variant={merging ? "secondary" : "primary"}
            disabled={disabled}
            onClick={onApprove}
          >
            {approveLabel}
          </Button>
        </Tooltip>
        {/* Off while the note is blank, which is what Fleet would answer. */}
        <Tooltip label={requestChangesNote}>
          <Button
            variant="secondary"
            disabled={disabled || blank}
            onClick={onRequestChanges}
          >
            {requestChangesLabel}
          </Button>
        </Tooltip>
      </div>

      {/* The rule is load-bearing, not decoration: it is what says the control
          under it is not another answer in the group above. */}
      <Separator decorative={false} className="armada-decision__rule" />

      <div className="armada-decision__terminal">
        {/* Outlined, because a solid red control reads as an error state rather
            than as an act. Alone, because it is the only one of the three that
            leaves nothing behind. */}
        <Tooltip label={rejectNote}>
          <Button variant="destructive" disabled={disabled} onClick={onReject}>
            {rejectLabel}
          </Button>
        </Tooltip>
      </div>

      {disabled && disabledNote !== undefined ? (
        <p className="armada-decision__said" role="note">
          {disabledNote}
        </p>
      ) : null}
    </div>
  );
}
