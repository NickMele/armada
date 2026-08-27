import type { ReactNode } from "react";

/**
 * Job brief — what the Job was told, and what done means for it.
 *
 * **Both halves sit beside where the work is**, rather than in a region of
 * their own: a person chasing a stopped Job asks what it was asked to do in
 * the same breath as where its files are, and two regions would separate the
 * question from the answer.
 *
 * **Every criterion carries its number, and the order is the frozen order.**
 * The contract sorts refusals first; nothing serves a verdict per criterion
 * yet, so there is nothing to sort by and the `acceptance_criteria[]` order is
 * what a citation to "criterion 4" resolves against.
 *
 * **No verdict hue, because no verdict is served.** A criterion here is what
 * was asked, not what was ruled — the day a ruling arrives it is per criterion
 * and never sums onto the step or the Job.
 *
 * The source is the verification source — the closed vocabulary of three. It
 * renders as the wire spells it: no registry carries a verb for
 * `criterion_source`, and one written here would be a second vocabulary.
 */
export type JobBriefCriterion = {
  /**
   * What was asked, in the words it was asked in. `criterion_id` is not drawn
   * beside it: the contract makes the row's number what a citation resolves
   * against, and a ULID on every row is noise a reader has to step over.
   */
  text: ReactNode;
  /** `check`, `judge` or `attested`, as the wire spells it. */
  source?: ReactNode;
};

export type JobBriefProps = {
  criteria: JobBriefCriterion[];
  /** Why there are none, where there are none. Never a labelled blank. */
  criteriaAbsent?: ReactNode;
  /** The context the Job was given, in the words it was given in. */
  facts?: ReactNode;
  /** Why there are none, where there are none. */
  factsAbsent?: ReactNode;
  criteriaLabel?: ReactNode;
  factsLabel?: ReactNode;
};

export function JobBrief({
  criteria,
  criteriaAbsent,
  facts,
  factsAbsent,
  criteriaLabel = "Done means",
  factsLabel = "What it was told",
}: JobBriefProps) {
  return (
    <div className="armada-job-brief">
      <div className="armada-job-brief__block">
        <span className="armada-job-brief__label">{criteriaLabel}</span>
        {criteria.length === 0 ? (
          <p className="armada-job-brief__note">{criteriaAbsent}</p>
        ) : (
          <ol className="armada-job-brief__criteria">
            {criteria.map((criterion, i) => (
              <li className="armada-job-brief__criterion" key={i}>
                <span className="armada-job-brief__ordinal">{i + 1}</span>
                <span className="armada-job-brief__text">{criterion.text}</span>
                {criterion.source === undefined ? (
                  <span />
                ) : (
                  <span className="armada-job-brief__source">{criterion.source}</span>
                )}
              </li>
            ))}
          </ol>
        )}
      </div>

      <div className="armada-job-brief__block">
        <span className="armada-job-brief__label">{factsLabel}</span>
        {facts === undefined ? (
          <p className="armada-job-brief__note">{factsAbsent}</p>
        ) : (
          <p className="armada-job-brief__facts">{facts}</p>
        )}
      </div>
    </div>
  );
}
