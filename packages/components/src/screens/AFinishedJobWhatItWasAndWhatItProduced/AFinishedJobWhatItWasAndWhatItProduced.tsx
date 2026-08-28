import { JobBrief, type JobBriefProps } from "../../compositions/JobBrief/JobBrief";
import { JobDetailHeaderActions } from "../../compositions/JobDetailHeaderActions/JobDetailHeaderActions";
import { JobOutcome, type JobOutcomeProps } from "../../compositions/JobOutcome/JobOutcome";
import { JobRecord, type JobRecordSection } from "../../compositions/JobRecord/JobRecord";
import { Absent } from "../absent";
import type { JobDetailHeading } from "../detail";

/**
 * A finished job — what it was, and what it produced.
 *
 * **A finished Job is read once, to decide whether to take the work.** That is
 * a different act from watching one, and it wants a different page. Step state,
 * per-step elapsed and a live rail answer "where is it now", which nobody asks
 * once a Job stopped — so this screen answers two questions at full weight and
 * folds everything else into a record one interaction away.
 *
 * | | |
 * |---|---|
 * | What this was | The header's title and fact run, then what done meant |
 * | What came out | The branch, and every other part of "produced" |
 * | The record | Steps, checks, turns, context, paths — folded |
 *
 * **The brief is split across the two.** Its two halves normally sit together,
 * because a person chasing a stopped Job asks what it was told and where its
 * files are in one breath. Here they answer different questions: what done
 * meant is the first thing asked of a finished Job, and the context it was
 * given is part of the record. Neither half is drawn twice.
 *
 * **Nothing on this page is a summary.** Every value is a fact Fleet served or
 * a path derived from one. A prose summary of what the Drone did would be the
 * Drone's own account presented as the record, which is the distinction the
 * whole verification loop exists to hold.
 *
 * **The pull request is the review surface; this is the decision surface.**
 * When Fleet opens one, this page's job is to get a person to it with enough
 * context to know whether to look — not to compete with it.
 */
export type AFinishedJobWhatItWasAndWhatItProducedProps = {
  heading: JobDetailHeading;
  /**
   * What done meant. The criteria half of the brief: this screen passes
   * `only="criteria"`, and the context half belongs to a record section.
   */
  brief?: JobBriefProps;
  /** Why there is no brief to draw, where there is none. */
  briefAbsent?: string;
  /** The branch, and every other part of what was produced. */
  outcome?: JobOutcomeProps;
  /** Why there is no outcome to draw, where there is none. */
  outcomeAbsent?: string;
  /** Everything else, in the order a reader asks for it. */
  record?: JobRecordSection[];
  /** Which section is open. Controlled, so a section can own a subscription. */
  recordValue?: string;
  onRecordChange?: (id: string) => void;
  /** What the record says when it holds no section at all. */
  recordAbsent?: string;
  /** The labels over the three regions. */
  wasLabel?: string;
  producedLabel?: string;
  recordLabel?: string;
  onCopied?: (value: string) => void;
};

export function AFinishedJobWhatItWasAndWhatItProduced({
  heading,
  brief,
  briefAbsent = "Nothing serves this Job's acceptance criteria.",
  outcome,
  outcomeAbsent = "Nothing serves a branch or a worktree yet.",
  record = [],
  recordValue,
  onRecordChange,
  recordAbsent,
  wasLabel = "What this was",
  producedLabel = "What came out",
  recordLabel = "The record",
  onCopied,
}: AFinishedJobWhatItWasAndWhatItProducedProps) {
  return (
    <div className="armada-screen__detail">
      <JobDetailHeaderActions {...heading} onCopied={onCopied} />

      <div className="armada-screen__col">
        <span className="armada-screen__eyebrow">{wasLabel}</span>
        {brief === undefined ? (
          <div className="armada-screen__slot">
            <Absent name="What this was" note={briefAbsent} />
          </div>
        ) : (
          <JobBrief {...brief} only="criteria" />
        )}
      </div>

      <div className="armada-screen__col">
        <span className="armada-screen__eyebrow">{producedLabel}</span>
        {outcome === undefined ? (
          <div className="armada-screen__slot">
            <Absent name="What came out" note={outcomeAbsent} />
          </div>
        ) : (
          <JobOutcome {...outcome} onCopied={onCopied} />
        )}
      </div>

      <div className="armada-screen__col">
        <span className="armada-screen__eyebrow">{recordLabel}</span>
        <JobRecord
          sections={record}
          value={recordValue}
          onChange={onRecordChange}
          emptyNote={recordAbsent}
        />
      </div>
    </div>
  );
}
