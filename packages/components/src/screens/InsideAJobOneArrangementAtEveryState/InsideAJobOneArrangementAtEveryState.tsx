import type { ReactNode } from "react";
import { JobBrief, type JobBriefProps } from "../../compositions/JobBrief/JobBrief";
import { JobDetailHeaderActions } from "../../compositions/JobDetailHeaderActions/JobDetailHeaderActions";
import type { JobDetailField } from "../../compositions/JobDetailHeaderActions/JobDetailHeaderActions";
import { JobLogReference, type JobLogReferenceRow } from "../../compositions/JobLogReference/JobLogReference";
import { PhaseStrip, type PhaseStripProps } from "../../compositions/PhaseStrip/PhaseStrip";
import { RunTree, type RunTreeStep } from "../../compositions/RunTree/RunTree";
import { StepStory, type StepChapter } from "../../compositions/StepStory/StepStory";
import { Absent } from "../absent";
import type { JobDetailHeading } from "../detail";

/**
 * Inside a Job — one arrangement, at every state.
 *
 * **This is the whole point of the screen.** Job detail had an arrangement per
 * state — running, awaiting review, failed, finished, and observing as its own
 * route — and below the header no region sat in the same place twice. Getting
 * to a Job was never the problem; being inside one was. So: the run as a tree
 * on the left, the selected step in the panel, and the step's story in the
 * order it happened. What changes between states is which chapter is the reason
 * you are here and what the panel offers you to do about it — never where a
 * region sits.
 *
 * **The tree and the panel divide the work, and building either alone loses the
 * rule.** The tree holds short facts; the panel holds anything that is a
 * sentence. That is why a step's `Produced` is three paths in the tree and a
 * diff in the panel, and why the tree's rows never grow prose.
 *
 * **Acts are split by what they act on.** An act that changes a step — restart
 * it, redirect it, overrule the verdict, re-run the gate — sits in the panel
 * header and takes the accent, because the object of attention on this screen
 * is the open step. An act that ends or replaces the Job — kill, redispatch,
 * approve — sits in the Job header. Four of the eight were rendered at Job
 * level and are not any more.
 *
 * **The Job header's action group is a slot, and Pilot lands in it** — secondary
 * on a running Job, primary on an escalated one, in the same place both times,
 * left of Kill. That is #250 and is not built here; nothing about this header
 * has to change to take it.
 *
 * **The brief sits above the step, on the panel's raised surface**, because
 * every step is read against it.
 */

/** The selected step, as the panel draws it. */
export type StepPanel = {
  /** The step's name, in sans. Nouns naming the artifact. */
  label: ReactNode;
  /** Whether `label` is a `step_id` rather than a name, so it renders in mono. */
  labelIsAnIdentifier?: boolean;
  /**
   * The step's short facts — `Running for 6m 11s`, `Attempt 2 of 3`, `Drone
   * alive, idle`. **Figures, never a chart**: a filled bar reads as progress
   * and a step has no percentage.
   */
  fields: JobDetailField[];
  /**
   * The acts that change this step. **They take the accent**, and they sit here
   * rather than in the Job header because they act on the step.
   */
  acts?: ReactNode;
  /**
   * The band above the story: what happened, and why you are looking at this
   * step. Absent on a step where nothing has gone wrong, which is most of them.
   */
  notice?: StepNotice;
  /** Where this step is — its phases and its gate tiers as one progression. */
  phases?: PhaseStripProps;
  /** Why there is no strip, where there is none. */
  phasesAbsent?: string;
  /**
   * Anything between the strip and the story — the failure every attempt hit,
   * what the Drone tried, the box that drafts a redirect. **It comes before the
   * story and after the strip** because you cannot write a useful sentence
   * until you have read them.
   */
  before?: ReactNode;
  /** Drone instructions, Activity log, Produced — in that order, always. */
  chapters: StepChapter[];
  /** Which chapter is open on mount. */
  openChapter?: string;
  /**
   * After the story — the decision, on a step waiting for one. **At the end
   * rather than in the header**, because you make it after reading; the header
   * is for acts that change what a Drone is doing.
   */
  after?: ReactNode;
};

/**
 * The band that says why you are here.
 *
 * **Its tone is a step-level token and never a Job status.** A failed Check is
 * `--step-failed`; a step holding with its retries spent is `--step-stopped-bg`;
 * a step waiting on a person is `--step-waiting`, amber and never red, because
 * everything mechanical has cleared and that must not read as a failure.
 * `note` takes no hue at all.
 */
export type StepNotice = {
  tone: "failed" | "stopped" | "waiting" | "note";
  /** What happened, in one line. */
  title?: ReactNode;
  children: ReactNode;
};

export type InsideAJobProps = {
  heading: JobDetailHeading;
  /** The run, in order. One row per step of the frozen workflow. */
  run: RunTreeStep[];
  runLabel?: ReactNode;
  /** The whole Job's elapsed, beside the label. A figure, never a chart. */
  runElapsed?: ReactNode;
  /** Why there is no run to draw, where there is none. */
  runAbsent?: string;
  /**
   * The running mark on the current step animates. One per screen: this is the
   * Job being read, so the tree pulses and the header badge stays static.
   */
  pulsing?: boolean;
  onSelectStep?: (stepId: string) => void;
  /**
   * Where things are — the worktree, the branch, the Manifest, the workflow,
   * the log, the transcript, the Drone. **A path opens where it lives; an
   * identifier copies.** This milestone is about never needing these; they are
   * here for when you want them anyway.
   */
  where?: JobLogReferenceRow[];
  whereLabel?: ReactNode;
  whereNote?: ReactNode;
  /** Why nothing can be named there, where nothing can. */
  whereAbsent?: string;
  /** The Job's brief, above the step on the panel's raised surface. */
  brief?: JobBriefProps;
  /** Why there is no brief, where there is none. */
  briefAbsent?: string;
  /** The step the panel is showing. */
  step?: StepPanel;
  /** Why no step is open, where none is. */
  stepAbsent?: string;
  onCopied?: (value: string) => void;
};

export function InsideAJob({
  heading,
  run,
  runLabel = "The run",
  runElapsed,
  runAbsent = "Nothing serves this Job's workflow, so its steps are unknown.",
  pulsing = true,
  onSelectStep,
  where,
  whereLabel = "Where things are",
  whereNote,
  whereAbsent = "Nothing serves this Job's paths or its branch.",
  brief,
  briefAbsent = "Nothing serves this Job's brief or its acceptance criteria.",
  step,
  stepAbsent = "No step is open. Select one in the run.",
  onCopied,
}: InsideAJobProps) {
  return (
    <div className="armada-screen__detail">
      <JobDetailHeaderActions {...heading} onCopied={onCopied} />

      <div className="armada-inside">
        {/* The run, and the pointers beneath it. Left, at every state. */}
        <div className="armada-inside__run">
          <div className="armada-inside__region-head">
            <span className="armada-screen__eyebrow">{runLabel}</span>
            {runElapsed === undefined ? null : (
              <span className="armada-inside__elapsed">{runElapsed}</span>
            )}
          </div>
          {run.length === 0 ? (
            <div className="armada-screen__slot">
              <Absent name="The run" note={runAbsent} />
            </div>
          ) : (
            <RunTree
              steps={run}
              pulsing={pulsing}
              onSelect={onSelectStep}
              onCopied={onCopied}
            />
          )}

          <span className="armada-screen__eyebrow" data-spaced>
            {whereLabel}
          </span>
          {where === undefined || where.length === 0 ? (
            <div className="armada-screen__slot">
              <Absent name="Where things are" note={whereAbsent} />
            </div>
          ) : (
            <JobLogReference rows={where} onCopied={onCopied}>
              {whereNote}
            </JobLogReference>
          )}
        </div>

        {/* The panel. Same regions in the same order at every state. */}
        <div className="armada-inside__panel">
          <div className="armada-inside__brief">
            <span className="armada-screen__eyebrow">Brief</span>
            {brief === undefined ? (
              <div className="armada-screen__slot">
                <Absent name="Brief" note={briefAbsent} />
              </div>
            ) : (
              <JobBrief {...brief} />
            )}
          </div>

          {step === undefined ? (
            <div className="armada-screen__slot">
              <Absent name="The step" note={stepAbsent} />
            </div>
          ) : (
            <>
              <div className="armada-inside__step-head">
                <div className="armada-inside__step-titles">
                  <span
                    className="armada-inside__step-name"
                    data-identifier={step.labelIsAnIdentifier || undefined}
                  >
                    {step.label}
                  </span>
                  <div className="armada-inside__step-fields">
                    {step.fields.map((field, f) => (
                      <span className="armada-inside__field" key={f}>
                        {field.label === undefined ? null : (
                          <span className="armada-inside__field-label">{field.label}</span>
                        )}
                        {field.value === undefined ? null : (
                          <span
                            className="armada-inside__field-value"
                            data-mono={field.mono || undefined}
                          >
                            {field.value}
                          </span>
                        )}
                      </span>
                    ))}
                  </div>
                </div>
                {/* The step acts, and the accent goes with them. */}
                {step.acts === undefined ? null : (
                  <div className="armada-inside__step-acts">{step.acts}</div>
                )}
              </div>

              {step.notice === undefined ? null : (
                <div className="armada-inside__notice" data-tone={step.notice.tone} role="status">
                  {step.notice.title === undefined ? null : (
                    <span className="armada-inside__notice-title">{step.notice.title}</span>
                  )}
                  <span className="armada-inside__notice-body">{step.notice.children}</span>
                </div>
              )}

              {step.phases === undefined ? (
                <p className="armada-inside__absent" role="note">
                  {step.phasesAbsent ??
                    "Nothing serves this step's gates, so where it stands is unknown."}
                </p>
              ) : (
                <PhaseStrip {...step.phases} />
              )}

              {step.before === undefined ? null : (
                <div className="armada-inside__before">{step.before}</div>
              )}

              <StepStory chapters={step.chapters} openId={step.openChapter} />

              {step.after === undefined ? null : (
                <div className="armada-inside__after">{step.after}</div>
              )}
            </>
          )}
        </div>
      </div>
    </div>
  );
}
