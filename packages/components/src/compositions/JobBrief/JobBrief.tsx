import type { ReactNode } from "react";
import { Skeleton } from "../../primitives/Skeleton/Skeleton";
import { Clamped } from "../Clamped/Clamped";

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
  /**
   * The sub-label over each half. **`null` draws no element at all**, not an
   * empty one: job detail names the region `Brief` and puts the sentence
   * beside it, and a blank span there is a line box of dead space above the
   * one line the region exists to show.
   */
  criteriaLabel?: ReactNode;
  factsLabel?: ReactNode;
  /**
   * How many lines of `facts` to show before the rest is a press away.
   *
   * **A brief has no ceiling and this region does.** It is the first thing in
   * the panel, above the step and its whole story, so a long one pushes what a
   * reader opened the Job for off the screen. Four lines is enough to tell one
   * Job from another, which is what a reader is doing when they glance here.
   */
  factsLines?: number;
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
  factsLines = 4,
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
          <Label>{waitingLabel}</Label>
          <p className="armada-job-brief__facts">{waiting}</p>
        </div>
      )}

      {only === "facts" ? null : (
        <div className="armada-job-brief__block">
          <Label>{criteriaLabel}</Label>
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
          <Label>{factsLabel}</Label>
          {facts === undefined ? (
            <p className="armada-job-brief__note">{factsAbsent}</p>
          ) : (
            // Held to a few lines. What a Job was told runs to whatever length
            // the person writing it needed, and it sits above the step, the
            // strip and the whole story — an unbounded paragraph here pushes
            // everything a reader opened the Job for off the screen. The
            // control only draws where there is more, so a short brief is
            // untouched.
            <Clamped lines={factsLines}>
              <p className="armada-job-brief__facts">{facts}</p>
            </Clamped>
          )}
        </div>
      )}
    </div>
  );
}

/**
 * A half's sub-label, or nothing where the caller passed `null`. The element
 * goes with the text: an empty span still occupies its line box, which is the
 * gap job detail had between the region's own label and the sentence under it.
 */
function Label({ children }: { children: ReactNode }) {
  if (children === null) return null;
  return <span className="armada-job-brief__label">{children}</span>;
}

/** How wide a loading criterion's text bar runs, one per row. */
const CRITERION_SKELETON_WIDTHS = ["80%", "60%"];
/** How wide each loading line of the brief's own words runs. Two: what a
 * real brief on this screen wraps to, not a round number. */
const FACTS_SKELETON_WIDTHS = ["90%", "60%"];

/**
 * The brief, before it has come back. **Same blocks, same grids** —
 * `.armada-job-brief__criteria`'s subgrid and `.armada-job-brief__facts`'
 * paragraph — reusing the real classes rather than a shape of its own, so
 * nothing moves once the criteria and the facts land.
 *
 * **No criteria block by default, and no facts label.** `screens/work.ts`'s
 * `briefOf` always sends `criteria: []` and `factsLabel: null` here — a
 * criterion's verdict is drawn where the Judge stage opens it, not in this
 * block — so a skeleton that shows either by default is guessing wrong on
 * every real Job this screen draws. `criteriaRows` is left as an escape
 * hatch rather than deleted outright, for a caller that one day knows a
 * count in advance.
 */
export function JobBriefSkeleton({
  criteriaRows = 0,
  criteriaLabel = "Done means",
  factsLabel = null,
}: {
  criteriaRows?: number;
  criteriaLabel?: ReactNode;
  factsLabel?: ReactNode;
}) {
  return (
    <div className="armada-job-brief" role="status" aria-label="Reading the brief" aria-busy>
      {criteriaRows === 0 ? null : (
        <div className="armada-job-brief__block">
          <Label>{criteriaLabel}</Label>
          <ol className="armada-job-brief__criteria">
            {CRITERION_SKELETON_WIDTHS.slice(0, criteriaRows).map((width, i) => (
              <li className="armada-job-brief__criterion" key={i}>
                <span className="armada-job-brief__ordinal">{i + 1}</span>
                <Skeleton width={width} />
                <span />
              </li>
            ))}
          </ol>
        </div>
      )}
      <div className="armada-job-brief__block">
        <Label>{factsLabel}</Label>
        <div className="armada-job-brief__facts armada-skeleton-text">
          {FACTS_SKELETON_WIDTHS.map((width, i) => (
            <Skeleton key={i} width={width} />
          ))}
        </div>
      </div>
    </div>
  );
}
