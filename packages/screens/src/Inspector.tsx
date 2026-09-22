// The inspector — the open step, read whole. A column beside the run wherever
// the window can pay for both, and the body of a sheet over it where it cannot.
//
// **One tree, drawn in one of two places.** A second copy per width would be
// the arrangement-per-state this screen exists to end, arriving through the
// back door of a breakpoint.

import type { ReactNode } from "react";
import { Unplug } from "lucide-react";
import {
  JobBrief,
  JobBriefSkeleton,
  Skeleton,
  StepTimeline,
  StepTimelineSkeleton,
  Tooltip,
  type JobBriefProps,
  type JobDetailField,
  type StepTimelineAttempt,
  type WorkflowDiagramStep,
} from "@armada/components";

import { Eyebrow, FieldLabel } from "./regions";

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
  /**
   * The step as one timeline: its phases in the order they happened, repeated
   * per attempt, each row holding the chapter that phase produced.
   *
   * **It replaced a strip above a story.** The two said the same thing twice — a
   * failed Check was a red node in one and a printed exit code in the other —
   * and this is the one reading.
   */
  timeline?: StepTimelineAttempt[];
  /** Why there is no timeline, where there is none. */
  timelineAbsent?: string;
  /** What the Job has changed, as its own panel beside the run. #1187. */
  produced?: ReactNode;
  /** Which row is open, held by the surface, so a keyboard map can name one. */
  openRow?: string | null;
  onOpenRow?: (rowId: string | null) => void;
  /** Whether the timeline starts folded. `StepTimeline`'s own `folded`. */
  timelineFolded?: boolean;
  /**
   * Anything above the timeline — the failure every attempt hit, what the Drone
   * tried, the box that drafts a redirect. **It comes before the story**
   * because you cannot write a useful sentence until you have read it.
   */
  before?: ReactNode;
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
  /**
   * What the title means for a person, on hover over it. Prose, and never the
   * evidence: a list of what was refused or flagged stays in `children`.
   */
  says?: ReactNode;
  children?: ReactNode;
};

/**
 * The panel while a Job waits for approval: what will run, in place of the
 * idle step view. #1149 — nothing has run, so there is nothing to draw a
 * result for, only what somebody is agreeing to.
 */
export type StepOverview = {
  diagram: WorkflowDiagramStep[];
  /** Where the Job stops, its caps and its model — beside the diagram. */
  facts: JobDetailField[];
};

/** The panel while its step is read: what is already known of the step. */
export type StepReading = {
  /** The step's name off the workflow. Absent draws a bar in its place. */
  label?: ReactNode;
  labelIsAnIdentifier?: boolean;
  /** The phases every step is read against, by name. */
  phases: readonly ReactNode[];
};

export type InspectorProps = {
  /** Whether the inspector is folded into a sheet rather than drawn as a column. */
  folded?: boolean;
  brief?: JobBriefProps;
  briefAbsent: string;
  briefLoading: boolean;
  step?: StepPanel;
  stepAbsent: string;
  stepReading?: StepReading;
  overview?: StepOverview;
  unreachable?: string;
};

export function Inspector({
  folded = false,
  brief,
  briefAbsent,
  briefLoading,
  step,
  stepAbsent,
  stepReading,
  overview,
  unreachable,
}: InspectorProps) {
  return (
    <div className="armada-inside__panel" data-folded={folded || undefined}>
      {unreachable !== undefined ? null : (
        <div className="armada-inside__brief">
          <Eyebrow>Brief</Eyebrow>
          {briefLoading ? (
            <JobBriefSkeleton />
          ) : brief === undefined ? (
            <p className="armada-inside__absent" role="note">
              {briefAbsent}
            </p>
          ) : (
            <JobBrief {...brief} />
          )}
        </div>
      )}

      {stepReading !== undefined ? (
        <StepPanelReading {...stepReading} />
      ) : unreachable !== undefined ? (
        <div className="armada-inside__unreachable" role="status">
          <Unplug size={20} strokeWidth={1.5} aria-hidden />
          <span>{unreachable}</span>
        </div>
      ) : overview !== undefined ? (
        <div className="armada-inside__overview">
          <div className="armada-inside__step-fields">
            {overview.facts.map((field, f) => (
              <span className="armada-inside__field" key={f}>
                {field.label === undefined ? null : <FieldLabel>{field.label}</FieldLabel>}
                {field.value === undefined ? null : (
                  <span className="armada-inside__field-value" data-mono={field.mono || undefined}>
                    {field.value}
                  </span>
                )}
              </span>
            ))}
          </div>
        </div>
      ) : step === undefined ? (
        <p className="armada-inside__absent" role="note">
          {stepAbsent}
        </p>
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
                    {field.label === undefined ? null : <FieldLabel>{field.label}</FieldLabel>}
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
              {step.notice.title === undefined ? null : step.notice.says === undefined ? (
                <span className="armada-inside__notice-title">{step.notice.title}</span>
              ) : (
                <Tooltip asChild label={step.notice.says}>
                  <span className="armada-inside__notice-title">{step.notice.title}</span>
                </Tooltip>
              )}
              {step.notice.children === undefined ? null : (
                <span className="armada-inside__notice-body">{step.notice.children}</span>
              )}
            </div>
          )}

          {/* The box a person acts in comes before the story: you cannot
              write a useful sentence until you have read it. */}
          {step.before === undefined ? null : (
            <div className="armada-inside__before">{step.before}</div>
          )}

          {step.timeline === undefined ? (
            <p className="armada-inside__absent" role="note">
              {step.timelineAbsent ?? "Gates unknown"}
            </p>
          ) : (
            <StepTimeline
              attempts={step.timeline}
              label="Where this step is"
              openRow={step.openRow}
              onOpenRow={step.onOpenRow}
              folded={step.timelineFolded}
            />
          )}

          {step.produced}

          {step.after === undefined ? null : (
            <div className="armada-inside__after">{step.after}</div>
          )}
        </>
      )}
    </div>
  );
}

/**
 * The panel while its step is read: the step's name where the workflow gives
 * it, and its fields, gates and story waiting. The head is the real one, so
 * nothing moves when the step lands.
 */
function StepPanelReading({ label, labelIsAnIdentifier, phases }: StepReading) {
  return (
    <>
      <div className="armada-inside__step-head">
        <div className="armada-inside__step-titles">
          {label === undefined ? (
            <Skeleton width="40%" />
          ) : (
            <span className="armada-inside__step-name" data-identifier={labelIsAnIdentifier || undefined}>
              {label}
            </span>
          )}
          <div className="armada-inside__step-fields" role="status" aria-label="Reading the step" aria-busy>
            {/* Token widths: the row is as wide as its fields, so a percentage is of nothing. */}
            <Skeleton width="var(--space-12)" />
            <Skeleton width="var(--space-8)" />
          </div>
        </div>
      </div>
      <StepTimelineSkeleton phases={phases} />
    </>
  );
}
