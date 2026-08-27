import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";
import { Fragment } from "react";

/**
 * Criterion verdicts — what the Judge answered, beneath the step it judged.
 *
 * **A refusal is not a failed Check and must not look like one.** A Check says
 * the work is broken; a refusal says the work runs and is not what was asked
 * for. That difference is why one ends a Job and the other escalates it, and
 * the surface carries it three ways: the `circle-*` family instead of
 * `shield-*`, the criterion's own words instead of a command, and three
 * labelled lines of citation instead of an exit code.
 *
 * **A verdict is a measured fact and renders as flatly as one.** Hue is on the
 * glyph and the verb, per criterion, and never sums onto the step or the Job —
 * which is what lets a red cross sit under a running step beneath an escalated
 * badge without the three contradicting each other.
 *
 * **Refusals sort first and every row carries its number.** The number is the
 * criterion's frozen position in `acceptance_criteria[]`, so a citation to
 * "criterion 4" still resolves after the rows reorder. Sorted here rather than
 * by the caller: it is a rule about how this reads, and a caller that forgot it
 * would bury the row the screen was opened for.
 *
 * A `met` row is one line. Nothing more is owed — a step that passed its Judge
 * is an ordinary advanced step, and the design is spent on the refusal.
 */
export type CriterionVerdict = {
  /**
   * The criterion's frozen position in `acceptance_criteria[]`, 1-based. What
   * a citation names. Absent where the Job carries no criterion with this id,
   * because a position guessed from the row's place on screen would break the
   * one reference a retry is written against.
   */
  ordinal?: number;
  /** The criterion id, in mono. What the wire joins on. */
  criterionId: string;
  /** The requester's own words. Absent where the Job's criteria have no such id. */
  text?: ReactNode;
  /** `met` or `not_met`, for the hue. Never a word written at a call site. */
  named: string;
  /** The verb, from `criterion_verdict_judge` — "no objection", "refused". */
  verdict?: ReactNode;
  /** The glyph, from the `circle-*` family the Judge owns. */
  icon?: LucideIcon;
  /** What should be seen if the work were right. A refusal owes it. */
  expected?: ReactNode;
  /** What is seen instead. */
  produced?: ReactNode;
  /** What that difference does to whoever consumes it. The triage line. */
  consequence?: ReactNode;
};

export type CriterionVerdictsProps = {
  rows: CriterionVerdict[];
  /**
   * The label over the block, where it stands on its own. Absent inside a rail,
   * where the step row above already says what these are about.
   */
  label?: ReactNode;
};

/** Verdict glyphs are 12px at strokeWidth 2, like every mark below Job level. */
const VERDICT_ICON = 12;
const VERDICT_STROKE = 2;

/** What each cited field is called. The Judge record's own three names. */
const CITED: readonly ["expected", "produced", "consequence"] = [
  "expected",
  "produced",
  "consequence",
];

const LABELLED: Record<(typeof CITED)[number], string> = {
  expected: "Expected",
  produced: "Produced",
  consequence: "Consequence",
};

/** Refusals first, and everything else in the order it was asked. */
function refusalsFirst(rows: CriterionVerdict[]): CriterionVerdict[] {
  return [...rows].sort((a, b) => Number(b.named === "not_met") - Number(a.named === "not_met"));
}

export function CriterionVerdicts({ rows, label }: CriterionVerdictsProps) {
  return (
    <div className="armada-verdicts">
      {label ? <span className="armada-verdicts__label">{label}</span> : null}
      <ul className="armada-verdicts__list">
        {refusalsFirst(rows).map((row) => {
          const cited = CITED.filter((field) => row[field] !== undefined);
          return (
            <li className="armada-verdicts__row" key={row.criterionId} data-verdict={row.named}>
              <div className="armada-verdicts__head">
                <span className="armada-verdicts__mark">
                  {row.icon ? (
                    <row.icon size={VERDICT_ICON} strokeWidth={VERDICT_STROKE} aria-hidden />
                  ) : null}
                </span>
                {/* The number is the citation's anchor, so it renders whether
                    or not the criterion's words reached this screen. */}
                {row.ordinal === undefined ? null : (
                  <span className="armada-verdicts__ordinal">{`${row.ordinal}.`}</span>
                )}
                {row.text === undefined ? (
                  <span className="armada-verdicts__id">{row.criterionId}</span>
                ) : (
                  <span className="armada-verdicts__text">{row.text}</span>
                )}
                {row.verdict ? (
                  <span className="armada-verdicts__verb">{row.verdict}</span>
                ) : null}
              </div>
              {cited.length === 0 ? null : (
                // A refusal's citation, in the Judge record's own field names.
                // Three labelled lines rather than one sentence: the fields
                // arrive named, and composing prose out of them here would be
                // writing copy the Judge did not.
                <dl className="armada-verdicts__cited">
                  {cited.map((field) => (
                    // Both halves are direct children of one grid, so the three
                    // values share a column edge. A wrapper per pair would give
                    // each its own grid and align nothing.
                    <Fragment key={field}>
                      <dt className="armada-verdicts__cite-label">{LABELLED[field]}</dt>
                      <dd className="armada-verdicts__cite-value" data-field={field}>
                        {row[field]}
                      </dd>
                    </Fragment>
                  ))}
                </dl>
              )}
            </li>
          );
        })}
      </ul>
    </div>
  );
}
