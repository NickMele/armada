import type { ReactNode } from "react";
import { ChangedFiles, type ChangedFilesProps } from "../../compositions/ChangedFiles/ChangedFiles";
import {
  DroneQuestion,
  type DroneQuestionProps,
} from "../../compositions/DroneQuestion/DroneQuestion";
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
 *
 * **A question sits above the split, full width.** Everything in the two
 * columns is what the drone has done; a question is the reason it has stopped
 * doing any of it, and a person who opened this job needs to see that before
 * they read a rail rather than after.
 *
 * **The footprint sits under the rail, in the wide column.** A drone that is
 * working and a drone that is thrashing look identical from the outside, and
 * the files it has touched are the cheapest thing that tells them apart — so it
 * reads beside what ran rather than below what came out. The wide column is
 * also the one that can hold a repository-relative path without truncating it,
 * which is the defect the narrow column would reintroduce.
 */
export type ARunningJobProps = {
  heading: JobDetailHeading;
  /**
   * The question this job's drone asked and nobody has answered yet.
   *
   * **Full width and above the split**, which is the one place on this screen a
   * region has ever been put: everything below it is what the drone has done,
   * and this is the reason it has stopped doing any of it. A person opening a
   * running job that is waiting on them must not have to find that out by
   * reading a rail.
   *
   * Absent is the ordinary case and there is no `absent` sentence beside it,
   * unlike every other region here. Those name what the wire does not carry;
   * this one is genuinely nothing, and a permanent "no question was asked" line
   * on every running job would be a region that never says anything.
   */
  question?: DroneQuestionProps;
  /** The steps, in order. `GET /workflows` is what fills this. */
  steps: WorkflowRailStep[];
  /** The label over the rail. */
  ranLabel?: ReactNode;
  /** Why there are no steps to draw, where there are none. */
  stepsAbsent?: string;
  /** What the drone has changed in its worktree, as of the last reading. */
  footprint?: ChangedFilesProps;
  /** The label over it. */
  footprintLabel?: ReactNode;
  /** Why there is no footprint to draw, where there is none. */
  footprintAbsent?: string;
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
  question,
  steps,
  ranLabel = "What ran",
  stepsAbsent = "Nothing serves this Job's workflow, so its steps are unknown.",
  footprint,
  footprintLabel = "Files changed",
  footprintAbsent = "Nothing has reported this drone's changed files yet.",
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

      {question === undefined ? null : <DroneQuestion {...question} />}

      <div className="armada-screen__split">
        <div className="armada-screen__col">
          <span className="armada-screen__eyebrow">{ranLabel}</span>
          {steps.length === 0 ? (
            <div className="armada-screen__slot">
              <Absent name="What ran" note={stepsAbsent} />
            </div>
          ) : (
            <WorkflowRail steps={steps} pulsing onCopied={onCopied} />
          )}

          <span className="armada-screen__eyebrow" data-spaced>
            {footprintLabel}
          </span>
          {footprint === undefined ? (
            <div className="armada-screen__slot">
              <Absent name="Files changed" note={footprintAbsent} />
            </div>
          ) : (
            /* Never pulsing. The rail already carries the one animated mark
               this screen is allowed, and it is on the more specific thing. */
            <ChangedFiles {...footprint} onCopied={onCopied} />
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
