import type { ReactNode } from "react";
import { JobBrief } from "../../compositions/JobBrief/JobBrief";
import { JobDetailHeaderActions } from "../../compositions/JobDetailHeaderActions/JobDetailHeaderActions";
import { JobLogReference } from "../../compositions/JobLogReference/JobLogReference";
import { WorkflowRail, type WorkflowRailStep } from "../../compositions/WorkflowRail/WorkflowRail";
import { Absent } from "../absent";
import type { JobDetailHeading, JobDetailLog } from "../detail";

/**
 * A failed job — a dead end, read as one. Four statements in order: what
 * failed, that the job is over, where the branch is, and where the log is.
 *
 * The header is the same block the running job renders. What changes with the
 * state is the field run and the trailing action — the acts on a dead end are
 * about its branch and its log and sit beside those below, so a terminal Job
 * carries nothing there. A Job that stopped and asked is not terminal, and its
 * one act is a redispatch, which does go there because it is about the Job.
 */
export type AFailedJobADeadEndReadAsOneProps = {
  heading: JobDetailHeading;
  /** Why this stopped, in the words the vocabulary supplies. */
  why?: ReactNode;
  /** Why there is no reason to state, where there is none. */
  whyAbsent?: string;
  /** The steps, in order. `GET /workflows` is what fills this. */
  steps: WorkflowRailStep[];
  /** The label over the rail. */
  ranLabel?: ReactNode;
  /** Why there are no steps to draw, where there are none. */
  stepsAbsent?: string;
  /** The failing check's tail, and its exit code beside the label. */
  output?: { tail: string; meta?: ReactNode };
  /** Why there is no check output, where there is none. */
  outputAbsent?: string;
  /** The brief, and the branch, worktree and log left in place. */
  work?: JobDetailLog;
  /** Why there is nothing to name there, where there is nothing. */
  workAbsent?: string;
  onCopied?: (value: string) => void;
};

export function AFailedJobADeadEndReadAsOne({
  heading,
  why,
  whyAbsent = "The Job carries no stored reason, and none is written here.",
  steps,
  ranLabel = "What ran",
  stepsAbsent = "Nothing serves this Job's workflow, so its steps are unknown.",
  output,
  outputAbsent = "Nothing serves a check's output yet.",
  work,
  workAbsent = "Nothing serves this Job's paths, its branch or its brief.",
  onCopied,
}: AFailedJobADeadEndReadAsOneProps) {
  return (
    <div className="armada-screen__detail">
      <JobDetailHeaderActions {...heading} onCopied={onCopied} />

      <div className="armada-screen__sunken">
        <span className="armada-screen__eyebrow">Why this stopped</span>
        {why === undefined ? (
          <div className="armada-screen__slot">
            <Absent name="Why this stopped" note={whyAbsent} />
          </div>
        ) : (
          <p className="armada-screen__why">{why}</p>
        )}
      </div>

      <div className="armada-screen__split" data-wide>
        <div className="armada-screen__col">
          <span className="armada-screen__eyebrow">{ranLabel}</span>
          {steps.length === 0 ? (
            <div className="armada-screen__slot">
              <Absent name="What ran" note={stepsAbsent} />
            </div>
          ) : (
            <WorkflowRail steps={steps} />
          )}
        </div>

        <div className="armada-screen__col" data-loose>
          <div className="armada-screen__col">
            <div className="armada-screen__head-row">
              <span className="armada-screen__eyebrow">Check output</span>
              {output?.meta ? <span className="armada-screen__tag">{output.meta}</span> : null}
            </div>
            {output === undefined ? (
              <div className="armada-screen__slot">
                <Absent name="Check output" note={outputAbsent} />
              </div>
            ) : (
              <pre className="armada-screen__output">{output.tail}</pre>
            )}
          </div>

          <div className="armada-screen__col">
            <span className="armada-screen__eyebrow">Where the work is</span>
            {work === undefined ? (
              <div className="armada-screen__slot">
                <Absent name="Where the work is" note={workAbsent} />
              </div>
            ) : (
              <>
                {work.brief === undefined ? null : <JobBrief {...work.brief} />}
                <JobLogReference rows={work.rows} onCopied={onCopied}>
                  {work.note}
                </JobLogReference>
                {work.actions ? (
                  <div className="armada-screen__actions">{work.actions}</div>
                ) : null}
              </>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
