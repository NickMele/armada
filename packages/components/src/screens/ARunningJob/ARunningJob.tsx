import type { ReactNode } from "react";
import { EvidenceCard, type EvidenceCardProps } from "../../compositions/EvidenceCard/EvidenceCard";
import { JobBrief } from "../../compositions/JobBrief/JobBrief";
import { JobDetailHeaderActions } from "../../compositions/JobDetailHeaderActions/JobDetailHeaderActions";
import { JobLogReference } from "../../compositions/JobLogReference/JobLogReference";
import { WorkflowRail, type WorkflowRailStep } from "../../compositions/WorkflowRail/WorkflowRail";
import { Absent } from "../absent";
import type { JobDetailHeading, JobDetailLog } from "../detail";

/**
 * A running job — what ran, the newest submission, and where the log is.
 *
 * **The pulse is on the rail, so the header badge is static.** The rail knows
 * which step is working and the badge only knows the Job is; one pulse per
 * screen goes on the more specific mark.
 *
 * `evidence` and `log` are optional because nothing serves either yet. An
 * absent one is named rather than dropped — a region that closes up reads as a
 * screen that is finished.
 */
export type ARunningJobProps = {
  heading: JobDetailHeading;
  /** The steps, in order. `GET /workflows` is what fills this. */
  steps: WorkflowRailStep[];
  /** The label over the rail. */
  ranLabel?: ReactNode;
  /** Why there are no steps to draw, where there are none. */
  stepsAbsent?: string;
  /** The newest work submission. */
  evidence?: EvidenceCardProps;
  /** Why there is no submission to draw, where there is none. */
  evidenceAbsent?: string;
  /** The brief, the worktree, the branch and the log. */
  log?: JobDetailLog;
  /** The label over it. */
  logLabel?: ReactNode;
  /** Why there is nothing to name there, where there is nothing. */
  logAbsent?: string;
  onCopied?: (value: string) => void;
};

export function ARunningJob({
  heading,
  steps,
  ranLabel = "What ran",
  stepsAbsent = "Nothing serves this Job's workflow, so its steps are unknown.",
  evidence,
  evidenceAbsent = "Nothing serves a work submission yet.",
  log,
  logLabel = "Where the work is",
  logAbsent = "Nothing serves this Job's paths, its branch or its brief.",
  onCopied,
}: ARunningJobProps) {
  return (
    <div className="armada-screen__detail">
      <JobDetailHeaderActions {...heading} onCopied={onCopied} />

      <div className="armada-screen__split">
        <div className="armada-screen__col">
          <span className="armada-screen__eyebrow">{ranLabel}</span>
          {steps.length === 0 ? (
            <div className="armada-screen__slot">
              <Absent name="What ran" note={stepsAbsent} />
            </div>
          ) : (
            <WorkflowRail steps={steps} pulsing />
          )}
        </div>

        <div className="armada-screen__col">
          <span className="armada-screen__eyebrow">Evidence so far</span>
          {evidence === undefined ? (
            <div className="armada-screen__slot">
              <Absent name="Evidence" note={evidenceAbsent} />
            </div>
          ) : (
            <EvidenceCard {...evidence} />
          )}
          <span className="armada-screen__eyebrow" data-spaced>
            {logLabel}
          </span>
          {log === undefined ? (
            <div className="armada-screen__slot">
              <Absent name="Where the work is" note={logAbsent} />
            </div>
          ) : (
            <div className="armada-screen__col">
              {log.brief === undefined ? null : <JobBrief {...log.brief} />}
              <JobLogReference rows={log.rows} actions={log.actions} onCopied={onCopied}>
                {log.note}
              </JobLogReference>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
