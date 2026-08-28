import type { ReactNode } from "react";
import { JobBrief } from "../../compositions/JobBrief/JobBrief";
import { JobDetailHeaderActions } from "../../compositions/JobDetailHeaderActions/JobDetailHeaderActions";
import { JobLogReference } from "../../compositions/JobLogReference/JobLogReference";
import { JobRecord, type JobRecordSection } from "../../compositions/JobRecord/JobRecord";
import { WorkflowRail, type WorkflowRailStep } from "../../compositions/WorkflowRail/WorkflowRail";
import { Absent } from "../absent";
import type { JobDetailHeading, JobDetailLog } from "../detail";

/**
 * A failed job — a dead end, read as one. What stopped it and whether anything
 * resumes it, then what ran, then where the work is, then the record.
 *
 * **This screen is the one a person opens with a question.** A Job that landed
 * is read once to decide whether to take the work; a Job that stopped is opened
 * to find out why, and every answer to that is here rather than in a database,
 * a transcript on disk or the source.
 *
 * **It does not take the finished screen's arrangement.** That one leads with
 * what a Job was and what it produced, and a Job that stopped produced nothing —
 * so the second region would be empty on every one of them. What leads here is
 * the pair of questions this state actually raises: what stopped it, and is it
 * recoverable. The archive comes after both are answered.
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
  /**
   * What still resumes this Job, or that nothing does. **Read beneath the
   * reason and never as a control**: the acts live in the header, and this is
   * what says which of them Fleet will take before one is pressed.
   */
  recourse?: ReactNode;
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
  /**
   * Everything the Job left behind, folded: the moves it made, its Drone's
   * turns, what it changed, what it claimed, what it was told.
   *
   * **After the diagnosis and not instead of it.** The rail above stays at full
   * weight because it is what a person triages on; these are the reads that
   * answer the next question, and each costs something to make, so only the
   * open one is drawn.
   */
  record?: JobRecordSection[];
  /** Which section is open. Controlled, so a section can own a subscription. */
  recordValue?: string;
  onRecordChange?: (id: string) => void;
  /** What the record says when it holds no section at all. */
  recordAbsent?: string;
  recordLabel?: string;
  onCopied?: (value: string) => void;
};

export function AFailedJobADeadEndReadAsOne({
  heading,
  why,
  whyAbsent = "The Job carries no stored reason, and none is written here.",
  recourse,
  steps,
  ranLabel = "What ran",
  stepsAbsent = "Nothing serves this Job's workflow, so its steps are unknown.",
  output,
  outputAbsent = "Nothing serves a check's output yet.",
  work,
  workAbsent = "Nothing serves this Job's paths, its branch or its brief.",
  record,
  recordValue,
  onRecordChange,
  recordAbsent,
  recordLabel = "What it left behind",
  onCopied,
}: AFailedJobADeadEndReadAsOneProps) {
  return (
    <div className="armada-screen__detail">
      <JobDetailHeaderActions {...heading} onCopied={onCopied} />

      {/* One block and not two. "Why did it stop" and "does anything resume
          it" are asked in the same breath, and a person who reads the first
          and not the second is the person who presses a button to find out. */}
      <div className="armada-screen__sunken">
        <span className="armada-screen__eyebrow">Why this stopped</span>
        {why === undefined ? (
          <div className="armada-screen__slot">
            <Absent name="Why this stopped" note={whyAbsent} />
          </div>
        ) : (
          <p className="armada-screen__why">{why}</p>
        )}
        {recourse === undefined ? null : (
          <p className="armada-screen__recourse">{recourse}</p>
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
            <WorkflowRail steps={steps} onCopied={onCopied} />
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

      {/* Omitted rather than drawn empty. A screen that always ends in a tab
          strip with nothing under it would be the hole this record exists to
          fill, one level up. */}
      {record === undefined ? null : (
        <div className="armada-screen__col">
          <span className="armada-screen__eyebrow">{recordLabel}</span>
          <JobRecord
            sections={record}
            value={recordValue}
            onChange={onRecordChange}
            emptyNote={recordAbsent}
          />
        </div>
      )}
    </div>
  );
}
