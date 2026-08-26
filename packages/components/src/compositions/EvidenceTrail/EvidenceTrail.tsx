import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

/**
 * Evidence trail — one entry per step, in submission order, with the Check
 * that let it pass.
 *
 * **The trail is the reason to open the screen, so it is the largest element
 * rather than a panel to expand.** If the trail is what a person came for, it
 * should not be the thing they have to open first. Whether it is worth reading
 * at all is the finding this milestone exists to produce: merging without
 * looking at it means the submission schema is wrong.
 *
 * **The three fields are the schema's, not a layout choice.** A work submission
 * carries `claimed` — what the work now does, as an observable — `shown_by`,
 * the artifact demonstrating it, and `not_claimed`, which is required and may
 * be empty. Rendering them as a paragraph would let a Drone report in prose,
 * which is the failure this milestone is watching for.
 *
 * **`not_claimed` always renders, and an empty one reads "Nothing".** A dash
 * would read as no answer, which is the reading the field exists to rule out.
 *
 * Hedge by source: `shown_by` names an artifact the system can point at, so it
 * is mono. `claimed` and `not_claimed` are the Drone's own words and render as
 * prose.
 */
export type EvidenceTrailEntry = {
  /** The step's name, in sans. A step is a unit of work with a name. */
  step: ReactNode;
  /**
   * The submission's provenance, in mono: the time, the `evidence_type`, and
   * the Checks that let it pass. Machine-derived, every part of it.
   */
  provenance?: ReactNode;
  /**
   * The glyph. The `file-*` family means evidence throughout, which is what
   * this row is.
   */
  icon?: LucideIcon;
  /** The accessible name for the glyph. */
  iconLabel?: string;
  /** What the work now does, as an observable. */
  claimed: ReactNode;
  /** The artifact demonstrating it — a diff, a command, a set of paths. Mono. */
  shownBy: ReactNode;
  /**
   * What the submission does not claim. Required, and may be empty — an empty
   * one renders the word rather than a dash.
   */
  notClaimed?: ReactNode;
};

export type EvidenceTrailProps = {
  entries: EvidenceTrailEntry[];
  /** The word an empty `not_claimed` renders. */
  emptyNotClaimed?: ReactNode;
};

/** Entry glyphs are 12px at strokeWidth 2, like every mark below Job level. */
const ENTRY_ICON = 12;
const ENTRY_STROKE = 2;

export function EvidenceTrail({ entries, emptyNotClaimed = "Nothing" }: EvidenceTrailProps) {
  return (
    <ol className="armada-evidence-trail">
      {entries.map((entry, i) => (
        <li className="armada-evidence-trail__entry" key={i}>
          <span className="armada-evidence-trail__mark">
            {entry.icon ? <entry.icon size={ENTRY_ICON} strokeWidth={ENTRY_STROKE} aria-hidden /> : null}
            {entry.iconLabel ? <span className="armada-evidence-trail__sr">{entry.iconLabel}</span> : null}
          </span>
          <div className="armada-evidence-trail__body">
            <div className="armada-evidence-trail__head">
              <span className="armada-evidence-trail__step">{entry.step}</span>
              {entry.provenance ? (
                <span className="armada-evidence-trail__provenance">{entry.provenance}</span>
              ) : null}
            </div>
            <div className="armada-evidence-trail__field">
              <span className="armada-evidence-trail__label">Claimed</span>
              <span className="armada-evidence-trail__value">{entry.claimed}</span>
            </div>
            <div className="armada-evidence-trail__field">
              <span className="armada-evidence-trail__label">Shown by</span>
              <span className="armada-evidence-trail__value" data-mono>
                {entry.shownBy}
              </span>
            </div>
            {/* Always rendered. An absent field and an empty one are different
                claims, and only one of them is a Drone saying "nothing". */}
            <div className="armada-evidence-trail__field">
              <span className="armada-evidence-trail__label">Not claimed</span>
              <span className="armada-evidence-trail__value">
                {entry.notClaimed ? entry.notClaimed : emptyNotClaimed}
              </span>
            </div>
          </div>
        </li>
      ))}
    </ol>
  );
}
