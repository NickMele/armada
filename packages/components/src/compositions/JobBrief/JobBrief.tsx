import type { ReactNode } from "react";

/**
 * Job brief — what the Job was told, what done means for it, and whatever is
 * still waiting to be told to it.
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
  /**
   * An instruction written for this Job that nothing has taken yet — a person's
   * reply at a gate, held until the next drone opens with it.
   *
   * **There is no `waitingAbsent`, and that is the point.** Every other half of
   * this block names its own absence, because a Job with no criteria and a Job
   * whose criteria failed to load look alike. Here absent is the ordinary
   * state of nearly every Job ever drawn, so a sentence explaining it would put
   * a permanent paragraph on every screen to report that nothing is happening.
   * The block is here while there is something to say and gone the rest of the
   * time.
   *
   * **It leads, and the two standing halves follow.** It is the newest thing
   * about the Job and the only one that will not be here later, so it reads
   * before the brief it is about to become part of.
   */
  waiting?: ReactNode;
  criteriaLabel?: ReactNode;
  factsLabel?: ReactNode;
  waitingLabel?: ReactNode;
  /**
   * Draw one half rather than both.
   *
   * **The default is both, and that is still the rule where a Job is being
   * chased.** The two halves sit together because "what was it asked to do" and
   * "where are its files" are asked in one breath. A finished Job is read once,
   * to decide whether to take the work, and there the two halves answer
   * different questions at different weights: what done meant is the first
   * thing asked, and the context it was given is part of the record. So the
   * finished render places them in two regions and this is what lets it.
   */
  only?: "criteria" | "facts";
};

export function JobBrief({
  criteria,
  criteriaAbsent,
  facts,
  factsAbsent,
  waiting,
  criteriaLabel = "Done means",
  factsLabel = "What it was told",
  waitingLabel = "Waiting to be told",
  only,
}: JobBriefProps) {
  return (
    <div className="armada-job-brief">
      {/* Outside `only`, because `only` chooses between the two standing
          halves and this is neither of them. A caller drawing one half of a
          Job's brief in a panel of its own still wants to know an instruction
          is sitting unread. */}
      {waiting === undefined ? null : (
        <div className="armada-job-brief__block" data-waiting>
          <span className="armada-job-brief__label">{waitingLabel}</span>
          <p className="armada-job-brief__facts">{waiting}</p>
        </div>
      )}

      {only === "facts" ? null : (
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
      )}

      {only === "criteria" ? null : (
        <div className="armada-job-brief__block">
          <span className="armada-job-brief__label">{factsLabel}</span>
          {facts === undefined ? (
            <p className="armada-job-brief__note">{factsAbsent}</p>
          ) : (
            <p className="armada-job-brief__facts">{facts}</p>
          )}
        </div>
      )}
    </div>
  );
}
