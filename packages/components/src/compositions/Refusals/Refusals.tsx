import type { ReactNode } from "react";

/**
 * One call a job reached for and was refused.
 *
 * **`detail` is the field a person reads.** The harness usually sends no reason
 * — the observed `permission_denied` line carried an empty `decision_reason` —
 * so a row drawn from `because` alone draws a column of blanks, which is the
 * whole of what was wrong: a job said it was blocked by policy and nothing on
 * screen said what the policy stopped.
 */
export type Refused = {
  /**
   * The tool that was reached for, in the harness's own spelling — `Bash`,
   * `Write`. **A label and not the finding**: it says which capability was
   * denied, and the command beside it says what to widen an allowlist to.
   */
  tool: string;
  /**
   * The command, the path, the pattern — the argument as the transcript
   * recorded it, whitespace collapsed and bounded by fleet at 200 characters.
   *
   * **Empty is a real answer and never a gap to fill.** A refusal whose
   * `called` row is not in the transcript — an adopted drone whose earlier
   * turns went into a pipe with no reader — has no command to name, and the row
   * says so rather than drawing a bare tool name. A `Bash` row naming no
   * command is the defect this component exists to close, and drawing one here
   * would be that defect in a new place.
   */
  detail: string;
  /**
   * That `detail` is less than what was sent, and how much there was —
   * *showing 200 of 14,320 characters*.
   *
   * **In the row and never in a tooltip.** A person reads this command to
   * decide what to widen an allowlist to, and a tooltip is not there at the
   * moment the command is being copied: a cut command pasted as a whole one is
   * the failure this row exists one level down from.
   *
   * **Its own line, never trailing the command.** Set inside the mono block it
   * would be selected and copied with the command it is a caveat about.
   *
   * Absent on a command that arrived whole, which is almost every row.
   */
  size?: ReactNode;
  /**
   * The harness's own wording, where it gave one.
   *
   * **Usually absent, and absent is the honest answer.** Nothing fills it in
   * from the trigger, and nothing here infers one — a reason invented from
   * `blocked_by_policy` would say the policy refused it, which is what the
   * person already read and could not act on.
   */
  because?: string;
};

export type RefusalsProps = {
  /**
   * The refusals, oldest first — **the earliest and not the last**. What
   * stopped a drone is what it reached for before it started working around
   * being stopped.
   *
   * **One row per refusal, never collapsed by command.** A drone refused the
   * same command five times is a drone that did not learn, which is the most
   * diagnostic thing on the screen; a count beside one row hides the retry and
   * reads as a single denial.
   *
   * Empty draws nothing at all — not a heading over a blank — because most
   * stopped jobs were refused nothing.
   */
  refused: Refused[];
  /**
   * What the rows are, said once over them rather than once on each. Omitted
   * where the surface already named them in the sentence above.
   */
  said?: ReactNode;
  /**
   * That the list is shorter than what happened, where it is —
   * *showing 50 of 137 refused calls*.
   *
   * **A size and never a warning.** A short list read as the whole one is worse
   * than no list, and the caller states the count because the wire carries it
   * beside the rows for exactly this. Absent is a list that is all of them,
   * which is the ordinary case.
   */
  note?: ReactNode;
  /**
   * What a fresh drone meets when it reaches for these again, and what would
   * change it.
   *
   * **The rows said what was stopped and nothing said whether it stays
   * stopped.** A person reading a refusal beside a restart button is being
   * asked to spend a drone on a repeat, because a toolset is rendered at spawn
   * and a restart renders the same one — so the list alone invites the press
   * that reproduces it.
   *
   * **Last, and under the size note rather than over it.** The note is a
   * caveat about the list; this is what follows from the list, and it sits
   * against whatever the surface says about the acts on offer.
   *
   * Absent draws nothing, which is a surface that has no reading of the rows.
   */
  again?: ReactNode;
};

/**
 * What a job reached for and was refused, as rows.
 *
 * **The trigger's evidence, drawn where the trigger is named.** `stopped_by`,
 * `recourse` and `worktree_on_disk` already render in the band above a stopped
 * step's story; this is the one thing that answers *unblock it from what*, and
 * it sits beside them for the reason the gaming check's findings do — a person
 * told a machine stopped their job needs what the machine stopped.
 *
 * **A command wraps and is never clipped.** Fleet bounds the argument at 200
 * characters with whitespace collapsed, so the longest row is a few lines in
 * the panel — and the tail of a command is the part that gets pasted into an
 * allowlist, so an ellipsis takes away the thing the row is for. Nothing here
 * is laid out to a width, so no row can scroll the panel sideways.
 *
 * **A row that was cut says so on a line of its own.** That is the same fact
 * the list's own note carries one level up, so the two are written in one
 * grammar rather than as two inventions: *showing 200 of 14,320 characters*
 * under a command, *showing 50 of 137 refused calls* under the list.
 *
 * **The list does not finish its own job, and `again` is what finishes it.** A
 * refusal drawn beside a restart button says what was stopped and says nothing
 * about whether restarting meets it again — so the rows read as a diagnosis
 * and the press reads as the cure, and it is not one. What is true is a
 * mechanism rather than advice, and the caller states it.
 */
export function Refusals({ refused, said, note, again }: RefusalsProps) {
  if (refused.length === 0) return null;
  return (
    <div className="armada-refusals">
      {said === undefined ? null : <span className="armada-refusals__said">{said}</span>}
      <ul className="armada-refusals__list">
        {refused.map((one, at) => (
          // The call id is not the key: it correlates two transcript rows and a
          // job with an unreadable transcript can carry the same id twice. The
          // list is ordered and never reordered, so its position is its name.
          <li className="armada-refusals__refusal" key={`refused-${at}`}>
            <span className="armada-refusals__tool">{one.tool}</span>
            {one.detail === "" ? (
              <span className="armada-refusals__unrecorded">
                The command it was running is not in the transcript.
              </span>
            ) : (
              <span className="armada-refusals__detail">{one.detail}</span>
            )}
            {one.size === undefined ? null : (
              <span className="armada-refusals__size">{one.size}</span>
            )}
            {one.because === undefined || one.because === "" ? null : (
              <span className="armada-refusals__because">{one.because}</span>
            )}
          </li>
        ))}
      </ul>
      {note === undefined ? null : <span className="armada-refusals__note">{note}</span>}
      {again === undefined ? null : <span className="armada-refusals__again">{again}</span>}
    </div>
  );
}
