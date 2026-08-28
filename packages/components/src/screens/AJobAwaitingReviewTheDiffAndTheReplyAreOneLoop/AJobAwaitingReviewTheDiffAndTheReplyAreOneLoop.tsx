import type { ReactNode } from "react";
import { EvidenceTrail, type EvidenceTrailProps } from "../../compositions/EvidenceTrail/EvidenceTrail";
import { JobBrief, type JobBriefProps } from "../../compositions/JobBrief/JobBrief";
import { JobDetailHeaderActions } from "../../compositions/JobDetailHeaderActions/JobDetailHeaderActions";
import { JobLogReference } from "../../compositions/JobLogReference/JobLogReference";
import { ReviewDecision, type ReviewDecisionProps } from "../../compositions/ReviewDecision/ReviewDecision";
import { UnifiedDiff, type UnifiedDiffProps } from "../../compositions/UnifiedDiff/UnifiedDiff";
import { Absent } from "../absent";
import type { JobDetailHeading, JobDetailLog } from "../detail";

/**
 * A job awaiting review — the diff and the reply are one loop.
 *
 * **The last of the seven steps in `docs/scope.md`**: *when the work is
 * complete he has a set of work he can review*. Until this existed every job
 * ended by sending its owner to a terminal, which is what the second abandoned
 * attempt — a CLI — was abandoned for.
 *
 * **Its own destination, not a section of the record.** A job at
 * `awaiting_review` is stopped and waiting on a person, so the running render's
 * live rail and per-step elapsed answer a question nobody is asking; and the
 * finished render is reached only once the job is over, by which point all
 * three acts are refused. The decision has a status of its own, so it gets a
 * screen of its own.
 *
 * | | |
 * |---|---|
 * | What was claimed | Every submission, step by step — the drone's own account |
 * | What was changed | The patch, as the repository rendered it |
 * | The decision | The note and the three answers, on the same surface |
 *
 * **The claims sit above the diff, and that order is the argument.** A work
 * submission is a signal and never the source of truth, so the reader has the
 * drone's account in hand and then reads the bytes that either bear it out or
 * do not. Putting the diff first would make the claims a summary of something
 * already read, which is the one thing evidence is not for.
 *
 * **The decision is on the page, beneath the diff, and never in a modal.**
 * `docs/practices/bridge.md`: a design that puts the reply in a separate route,
 * tab or modal from the diff recreates v1's problem inside Electron. There is
 * one scroll and one loop.
 *
 * **Nothing here is a summary.** Every value is a fact Fleet served or a path
 * derived from one. A prose account of what the drone did, presented as the
 * record, is the distinction the whole verification loop exists to hold.
 */
export type AJobAwaitingReviewTheDiffAndTheReplyAreOneLoopProps = {
  heading: JobDetailHeading;
  /** What done meant. The criteria half of the brief — the decision's own bar. */
  brief?: JobBriefProps;
  /** Why there is no brief to draw, where there is none. */
  briefAbsent?: string;
  /** Every submission, step by step, in submission order. */
  claims?: EvidenceTrailProps;
  /** Why there are no claims to draw, where there are none. */
  claimsAbsent?: string;
  /** The patch, as the repository rendered it. */
  diff?: UnifiedDiffProps;
  /** Why there is no diff to draw, where there is none. */
  diffAbsent?: string;
  /** The note and the three answers. Absent only where the read failed. */
  decision?: ReviewDecisionProps;
  /** Why there is no decision to offer, where there is none. */
  decisionAbsent?: string;
  /** Where the work is — the worktree, the branch, the log, the transcript. */
  work?: JobDetailLog;
  /** Why there is nothing to name there, where there is nothing. */
  workAbsent?: string;
  /** The labels over the regions. */
  briefLabel?: ReactNode;
  claimsLabel?: ReactNode;
  diffLabel?: ReactNode;
  decisionLabel?: ReactNode;
  workLabel?: ReactNode;
  onCopied?: (value: string) => void;
};

export function AJobAwaitingReviewTheDiffAndTheReplyAreOneLoop({
  heading,
  brief,
  briefAbsent = "Nothing serves this job's acceptance criteria.",
  claims,
  claimsAbsent = "Nothing serves this job's work submissions.",
  diff,
  diffAbsent = "Nothing serves this job's diff.",
  decision,
  decisionAbsent = "Fleet did not answer for this job, so there is nothing to decide on.",
  work,
  workAbsent = "Nothing serves this job's paths or its branch.",
  briefLabel = "What done meant",
  claimsLabel = "What the drone claimed",
  diffLabel = "What it changed",
  decisionLabel = "Your decision",
  workLabel = "Where the work is",
  onCopied,
}: AJobAwaitingReviewTheDiffAndTheReplyAreOneLoopProps) {
  return (
    <div className="armada-screen__detail">
      <JobDetailHeaderActions {...heading} onCopied={onCopied} />

      <div className="armada-screen__col">
        <span className="armada-screen__eyebrow">{briefLabel}</span>
        {brief === undefined ? (
          <div className="armada-screen__slot">
            <Absent name="What done meant" note={briefAbsent} />
          </div>
        ) : (
          <JobBrief {...brief} only="criteria" />
        )}
      </div>

      {/* The account first, then the bytes. A submission is a signal, and a
          reader holding it while reading the diff is doing the comparison the
          evidence exists to make possible. */}
      <div className="armada-screen__col">
        <span className="armada-screen__eyebrow" data-spaced>
          {claimsLabel}
        </span>
        {claims === undefined ? (
          <div className="armada-screen__slot">
            <Absent name="What the drone claimed" note={claimsAbsent} />
          </div>
        ) : (
          <EvidenceTrail {...claims} />
        )}
      </div>

      <div className="armada-screen__col">
        <span className="armada-screen__eyebrow" data-spaced>
          {diffLabel}
        </span>
        {diff === undefined ? (
          <div className="armada-screen__slot">
            <Absent name="What it changed" note={diffAbsent} />
          </div>
        ) : (
          <UnifiedDiff {...diff} onCopied={onCopied} />
        )}
      </div>

      {/* Beneath the diff and on the same page. One scroll, one loop. */}
      <div className="armada-screen__col">
        <span className="armada-screen__eyebrow" data-spaced>
          {decisionLabel}
        </span>
        {decision === undefined ? (
          <div className="armada-screen__slot">
            <Absent name="Your decision" note={decisionAbsent} />
          </div>
        ) : (
          <ReviewDecision {...decision} />
        )}
      </div>

      {/* Last, because it is what a person reaches for when the diff on screen
          is not enough — and the cut notice sends them here by name. */}
      <div className="armada-screen__col">
        <span className="armada-screen__eyebrow" data-spaced>
          {workLabel}
        </span>
        {work === undefined ? (
          <div className="armada-screen__slot">
            <Absent name="Where the work is" note={workAbsent} />
          </div>
        ) : (
          <div className="armada-screen__col">
            <JobLogReference rows={work.rows} actions={work.actions} onCopied={onCopied}>
              {work.note}
            </JobLogReference>
          </div>
        )}
      </div>
    </div>
  );
}
