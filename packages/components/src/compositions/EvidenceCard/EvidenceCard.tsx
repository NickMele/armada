import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

/**
 * Evidence card — one work submission, read while the job is still running.
 *
 * **The three fields are the tool's, not a layout choice.** A work submission
 * carries `claimed` — what the work now does, as an observable — `shown_by`,
 * the artifact demonstrating it, and `not_claimed`, everything the claim does
 * not assert. `crates/verification/src/submission.rs` takes exactly those
 * three and nothing else, and the labels here are those three words in
 * sentence case. Rendering them as a paragraph would let a Drone report in
 * prose, which is the failure this milestone is watching for.
 *
 * **`not_claimed` always renders, and an empty one reads "Nothing".** In the
 * schema it is not an `Option`: empty is a legal value and absent is not a
 * value at all. A dash would read as no answer, which is the reading the field
 * exists to rule out.
 *
 * **Hedge by source.** `shown_by` names an artifact Armada can reach — a file
 * set, a command and its exit code — so it is mono. `claimed` and
 * `not_claimed` are the Drone's own words and render as prose.
 *
 * The card is the trail's entry seen alone: on a running job there is one
 * submission per advanced step, and the newest is the whole reading. The
 * trail is what the same record becomes once the job is over.
 */
export type EvidenceCardProps = {
  /**
   * The glyph. The `file-*` family means evidence throughout, which is what
   * this card is.
   */
  icon?: LucideIcon;
  /** The accessible name for the glyph, since the text beside it is a step. */
  iconLabel?: string;
  /** The step the submission came from. A unit of work with a name, so sans. */
  step: ReactNode;
  /** When it was submitted. Machine-derived, so mono, and set back. */
  time?: ReactNode;
  /** What the work now does, as an observable. */
  claimed: ReactNode;
  /** The artifact demonstrating it. Mono. */
  shownBy: ReactNode;
  /** What the submission does not claim. Required, and may be empty. */
  notClaimed?: ReactNode;
  /** The word an empty `not_claimed` renders. */
  emptyNotClaimed?: ReactNode;
};

/** Card glyphs are 12px at strokeWidth 2, like every mark below Job level. */
const CARD_ICON = 12;
const CARD_STROKE = 2;

export function EvidenceCard({
  icon: Icon,
  iconLabel,
  step,
  time,
  claimed,
  shownBy,
  notClaimed,
  emptyNotClaimed = "Nothing",
}: EvidenceCardProps) {
  return (
    <div className="armada-evidence-card">
      <div className="armada-evidence-card__head">
        <span className="armada-evidence-card__mark">
          {Icon ? <Icon size={CARD_ICON} strokeWidth={CARD_STROKE} aria-hidden /> : null}
          {iconLabel ? <span className="armada-evidence-card__sr">{iconLabel}</span> : null}
        </span>
        <span className="armada-evidence-card__step">{step}</span>
        {time ? <span className="armada-evidence-card__time">{time}</span> : null}
      </div>
      <div className="armada-evidence-card__field">
        <span className="armada-evidence-card__label">Claimed</span>
        <span className="armada-evidence-card__value">{claimed}</span>
      </div>
      <div className="armada-evidence-card__field">
        <span className="armada-evidence-card__label">Shown by</span>
        <span className="armada-evidence-card__value" data-mono>
          {shownBy}
        </span>
      </div>
      {/* Always rendered. An absent field and an empty one are different
          claims, and only one of them is a Drone saying "nothing". */}
      <div className="armada-evidence-card__field">
        <span className="armada-evidence-card__label">Not claimed</span>
        <span className="armada-evidence-card__value">
          {notClaimed ? notClaimed : emptyNotClaimed}
        </span>
      </div>
    </div>
  );
}
