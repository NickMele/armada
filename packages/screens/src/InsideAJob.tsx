// The arrangement `JobDetail` renders. Its rules are in
// `packages/components/src/screens/screens.css`, which Bridge and Storybook
// both load.

import type { ReactNode } from "react";
import { Unplug } from "lucide-react";
import { Fragment, useCallback, useState } from "react";
import {
  JobBrief,
  JobBriefSkeleton,
  JobDetailHeaderActions,
  PhaseStrip,
  RunTree,
  RunTreeSkeleton,
  Skeleton,
  StepStory,
  Tooltip,
  WhereRow,
  WhereRowSkeleton,
  conceptSaid,
  type JobBriefProps,
  type JobDetailField,
  type JobDetailHeading,
  type JobLogReferenceRow,
  type NotOpened,
  type PhaseStripProps,
  type RunTreeStep,
  type StepChapter,
} from "@armada/components";

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
   * Which chapter is open, held by the surface. Present makes the story
   * controlled — see `StepStoryProps.openChapter`. It is here so a keyboard map
   * can name a chapter rather than find one by the class the story ships.
   */
  openChapterId?: string | null;
  /** Told when a chapter is opened or closed. */
  onOpenChapter?: (chapterId: string | null) => void;
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

export type InsideAJobProps = {
  heading: JobDetailHeading;
  /** The run, in order. One row per step of the frozen workflow. */
  run: RunTreeStep[];
  runLabel?: ReactNode;
  /** The whole Job's elapsed, beside the label. A figure, never a chart. */
  runElapsed?: ReactNode;
  /** Why there is no run to draw, where there is none. */
  runAbsent?: string;
  /** The run has been asked for and has not come back yet. Takes the slot
   * over `run`/`runAbsent`: a skeleton shaped like the tree, not a claim
   * about what is missing. */
  runLoading?: boolean;
  /**
   * The Job's pulse, in a few lines: the last thing anyone did on it, the process
   * count, the worktree, the disk. `JobHoldsSummary`, and the full reading is a
   * press away on the sheet, from `machineAct` on the title line.
   *
   * **Below the run and above the pointers.** It used to be a whole card above
   * the run, which is the largest thing in this column standing in front of the
   * thing a person opens a Job to read. It is context for the run, so it reads
   * after it.
   *
   * **It carries the latest event, and no second region does.** Cutting a
   * worktree, running a repository's preparation commands and reclaiming one
   * belong to no step, so Fleet's notes about them reach a reader here, beside
   * the Drone's turns and a person's moves, whichever came last.
   *
   * Absent draws nothing. A Job with nothing read and nothing recorded is not a
   * hole in the screen — the same rule `record` below keeps.
   */
  machine?: ReactNode;
  machineLabel?: ReactNode;
  /**
   * The control on the region's title line that opens the full reading. On the
   * title line and not in the block, where the run keeps its elapsed figure.
   */
  machineAct?: ReactNode;
  /**
   * The running mark on the current step animates. One per screen: this is the
   * Job being read, so the tree pulses and the header badge stays static.
   */
  pulsing?: boolean;
  onSelectStep?: (stepId: string) => void;
  /**
   * Which steps have their facts open, held by the surface. Present makes the
   * tree controlled — see `RunTreeProps.openSteps`. It is here so a keyboard
   * map can name a step rather than find its chevron by the class the tree
   * ships.
   */
  openSteps?: readonly string[];
  /** Told when a step's facts are opened or closed. */
  onOpenStep?: (stepId: string, open: boolean) => void;
  /**
   * Told when a fact in the tree names an artifact and is pressed — an
   * attempt's Check output, a step's patch.
   *
   * **The tree is where a reader notices something is wrong.** Making them find
   * the same fact again in the panel below to act on it is the navigation this
   * screen exists to remove, so the fact that says `test failed · exit 101`
   * opens the output of that attempt's run.
   */
  onOpenArtifact?: (artifactId: string) => void;
  /** Told when a fact in the tree names a chapter: select that step, on it. */
  onOpenChapter?: (stepId: string, chapterId: string) => void;
  /**
   * Where things are — the worktree, the branch, the Manifest, the workflow,
   * the log, the transcript, the Drone. **A path opens where it lives; an
   * identifier copies.**
   */
  where?: JobLogReferenceRow[];
  whereLabel?: ReactNode;
  /** Why nothing can be named there, where nothing can. */
  whereAbsent?: string;
  /** This region has been asked for and has not come back yet. Takes the
   * slot over `where`/`whereAbsent`. */
  whereLoading?: boolean;
  /**
   * Everything the Job left behind, folded — its moves, its Drone's turns, what
   * it touched, what it changed, what it claimed.
   *
   * **A Job-level region, in a Job-level column, at every state.** It sat in
   * the finished render with eight sections and the stopped render with five,
   * and the difference was never about the Job — it was about which screen a
   * status happened to route to. Absent draws nothing rather than an empty
   * frame: a Job with nothing recorded is not a hole in the screen.
   */
  record?: ReactNode;
  recordLabel?: ReactNode;
  /** The Job's brief, above the step on the panel's raised surface. */
  brief?: JobBriefProps;
  /** Why there is no brief, where there is none. */
  briefAbsent?: string;
  /** The brief has been asked for and has not come back yet. Takes the slot
   * over `brief`/`briefAbsent`. */
  briefLoading?: boolean;
  /** The step the panel is showing. */
  step?: StepPanel;
  /** Why no step is open, where none is. */
  stepAbsent?: string;
  /** The run has been asked for and has not come back yet, so which step
   * would open is not known either. Takes the slot over `step`/`stepAbsent`. */
  stepLoading?: boolean;
  /**
   * Why nothing about this Job could be read, where Fleet did not answer for
   * it. **One message where the step goes, instead of an empty region per
   * part of the screen**: every region would say the same thing, and four of
   * them read as four failures.
   */
  unreachable?: string;
  /**
   * The trailing sheet, where one is open — the step's activity log, or the
   * Job's patch.
   *
   * **It belongs to the screen and not to the window.** The layer is flush to
   * the screen's trailing edge and full height, so the run tree and the panel
   * stay on screen and under it: nothing navigated, and the chapter line a
   * reader came back to is still where it was. A window-fixed layer would cover
   * the shell's rail as well, which nothing asked it to.
   *
   * **The pulse goes with the reading.** With a sheet open the tree's current
   * step is behind the layer, so `pulsing` is what the caller turns off and the
   * sheet's own live mark takes it.
   */
  sheet?: ReactNode;
  onCopied?: (value: string) => void;
};

/** How wide each "where" row runs while the read is in flight. Five: the
 * baseline set `workOf` always pushes — Worktree, Branch, Manifest, Workflow,
 * Drone — before Job log and Transcript, which vary by state. */
const WHERE_SKELETON_WIDTHS = ["45%", "60%", "35%", "30%", "50%"];

export function InsideAJob({
  heading,
  run,
  runLabel = "The run",
  runElapsed,
  runAbsent = "Steps unknown",
  runLoading = false,
  machine,
  machineLabel = "Pulse",
  machineAct,
  pulsing = true,
  onSelectStep,
  openSteps,
  onOpenStep,
  onOpenArtifact,
  onOpenChapter,
  where,
  whereLabel = "Where things are",
  whereAbsent = "Paths unknown",
  whereLoading = false,
  record,
  recordLabel = "What it left behind",
  brief,
  briefAbsent = "No brief",
  briefLoading = false,
  step,
  stepAbsent = "Select a step in the run",
  stepLoading = false,
  unreachable,
  sheet,
  onCopied,
}: InsideAJobProps) {
  return (
    <div className="armada-screen__detail">
      <JobDetailHeaderActions {...heading} onCopied={onCopied} />

      <div className="armada-inside">
        {/* The run, and the pointers beneath it. Left, at every state. */}
        <div className="armada-inside__run">
          <div className="armada-inside__region-head">
            <Eyebrow>{runLabel}</Eyebrow>
            {runLoading ? (
              <Skeleton style={{ width: "3rem", height: "var(--text-xs)" }} />
            ) : runElapsed === undefined ? null : (
              <span className="armada-inside__elapsed">{runElapsed}</span>
            )}
          </div>
          {runLoading ? (
            <RunTreeSkeleton />
          ) : run.length === 0 ? (
            <p className="armada-inside__absent" role="note">
              {unreachable ?? runAbsent}
            </p>
          ) : (
            <RunTree
              steps={run}
              pulsing={pulsing}
              onSelect={onSelectStep}
              openSteps={openSteps}
              onOpen={onOpenStep}
              onCopied={onCopied}
              onOpenArtifact={onOpenArtifact}
              onOpenChapter={onOpenChapter}
            />
          )}

          {/* Context for the run, so it reads after it — and before the
              pointers, which is where you go once it says something is wrong. */}
          {machine === undefined ? null : (
            <>
              <div className="armada-inside__pulse-head">
                <Eyebrow>{machineLabel}</Eyebrow>
                {machineAct}
              </div>
              {machine}
            </>
          )}

          <Eyebrow spaced>{whereLabel}</Eyebrow>
          {whereLoading ? (
            <div
              className="armada-inside__where"
              role="status"
              aria-label="Reading where things are"
              aria-busy
            >
              {WHERE_SKELETON_WIDTHS.map((width, r) => (
                <WhereRowSkeleton key={r} valueWidth={width} />
              ))}
            </div>
          ) : where === undefined || where.length === 0 ? (
            <p className="armada-inside__absent" role="note">
              {unreachable ?? whereAbsent}
            </p>
          ) : (
            <WhereRegion rows={where} onCopied={onCopied} />
          )}

          {record === undefined ? null : (
            <>
              <Eyebrow spaced>{recordLabel}</Eyebrow>
              {record}
            </>
          )}
        </div>

        {/* The rule between the columns. Its own track, not a border on
            either side, so it measures the full height of the taller column
            whichever one that is. */}

        {/* The panel. Same regions in the same order at every state. */}
        <div className="armada-inside__panel">
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

          {stepLoading ? (
            <StepPanelSkeleton />
          ) : unreachable !== undefined ? (
            <div className="armada-inside__unreachable" role="status">
              <Unplug size={20} strokeWidth={1.5} aria-hidden />
              <span>{unreachable}</span>
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
                        {field.label === undefined ? null : (
                          <FieldLabel>{field.label}</FieldLabel>
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

              {step.phases === undefined ? (
                <p className="armada-inside__absent" role="note">
                  {step.phasesAbsent ??
                    "Gates unknown"}
                </p>
              ) : (
                // The screen's own handler falls through to the strip unless
                // the caller gave the strip one of its own, so a Check's row
                // in a phase card opens the same viewer every other selector
                // on this screen opens.
                <PhaseStrip onOpenArtifact={onOpenArtifact} {...step.phases} />
              )}

              {step.before === undefined ? null : (
                <div className="armada-inside__before">{step.before}</div>
              )}

              <StepStory
                chapters={step.chapters}
                openId={step.openChapter}
                openChapter={step.openChapterId}
                onOpen={step.onOpenChapter}
              />

              {step.after === undefined ? null : (
                <div className="armada-inside__after">{step.after}</div>
              )}
            </>
          )}
        </div>
      </div>

      {sheet}
    </div>
  );
}

/**
 * Shapes for the step panel's loading placeholder. Six phase chips and three
 * chapters, matching a typical step rather than a round number.
 */
const STEP_SKELETON_PHASES = ["5rem", "6rem", "4rem", "5.5rem"];
const STEP_SKELETON_CHAPTERS = ["Drone instructions", "Activity log", "Produced"];

/**
 * The step panel, before a step has come back. **Same head, same phase row,
 * same three chapters** — the name and its fields in `.armada-inside__step-*`,
 * a strip of `.armada-phases__node` chips, three closed `.armada-chapter`
 * shells — so the panel does not reflow once a step lands in these slots.
 */
function StepPanelSkeleton() {
  return (
    <div role="status" aria-label="Reading the step" aria-busy>
      <div className="armada-inside__step-head">
        <div className="armada-inside__step-titles">
          <Skeleton style={{ width: "40%", height: "var(--text-lg)" }} />
          <div className="armada-inside__step-fields">
            <Skeleton style={{ width: "5rem", height: "var(--text-2xs)" }} />
            <Skeleton style={{ width: "3rem", height: "var(--text-2xs)" }} />
          </div>
        </div>
      </div>
      <div className="armada-phases">
        <div style={{ display: "flex", gap: "var(--space-2)" }}>
          {STEP_SKELETON_PHASES.map((width, n) => (
            <div className="armada-phases__node" key={n}>
              <Skeleton width={width} />
            </div>
          ))}
        </div>
      </div>
      <div className="armada-story">
        {STEP_SKELETON_CHAPTERS.map((chapter) => (
          <section className="armada-chapter" key={chapter}>
            <div className="armada-chapter__line">
              <div className="armada-chapter__head">
                <Skeleton style={{ width: "var(--space-3)", height: "var(--text-2xs)" }} />
                <Skeleton style={{ width: "8rem", height: "var(--text-xs)" }} />
              </div>
            </div>
          </section>
        ))}
      </div>
    </div>
  );
}

/**
 * A region's band, and what that region is where its name is an Armada word.
 *
 * **The sentence is looked up rather than written here.** Six regions on this
 * screen name a thing rather than describe it — `Brief`, `The run`, `Where
 * things are` — and a screen that answered them itself would be a second place
 * the vocabulary is explained. `concepts.ts` is the first.
 *
 * `asChild`, because the brief's own rule keys off the band's class: a wrapper
 * would be the `:not(.armada-screen__eyebrow)` child and be styled as the
 * brief itself.
 */
function Eyebrow({ children, spaced }: { children: ReactNode; spaced?: boolean }) {
  const says = conceptSaid(children);
  const band = (
    <span className="armada-screen__eyebrow" data-spaced={spaced || undefined}>
      {children}
    </span>
  );
  return says === undefined ? (
    band
  ) : (
    <Tooltip asChild label={says}>
      {band}
    </Tooltip>
  );
}

/** A step field's label, on the same rule as the header's and the tree's. */
function FieldLabel({ children }: { children: ReactNode }) {
  const says = conceptSaid(children);
  const label = <span className="armada-inside__field-label">{children}</span>;
  return says === undefined ? (
    label
  ) : (
    <Tooltip asChild label={says}>
      {label}
    </Tooltip>
  );
}

/**
 * Where things are — a label column, the machine value, and the row's one act.
 *
 * **The label column is the region.** It was drawn by `JobLogReference`, which
 * has none: every row was a glyph and a path, and a reader had to work out from
 * the shape of a string whether it was a worktree, a branch or a transcript.
 * `WhereRow` was built for the drawn 74px column and nothing used it. This is
 * that composition, and the rows arrive in the shape the surface already builds
 * them in — the glyph each row carried is dropped, because the label it stood
 * in for is now written out.
 *
 * **An open can fail, and the row is where it says so.** That is the one thing
 * `WhereRow` cannot hold on its own: its act is synchronous, and whether a
 * worktree still exists is not known until the OS has been asked. So the region
 * holds the last refusal and draws it under the row it was pressed on — one at
 * a time, and it is the last press, because two stale rows arguing on screen is
 * worse than the one somebody just clicked.
 */
function WhereRegion({
  rows,
  onCopied,
}: {
  rows: JobLogReferenceRow[];
  onCopied?: (value: string) => void;
}) {
  const [unopened, setUnopened] = useState<{ row: number; because: string } | null>(null);
  /** The row with an open in flight. A second press does not send a second. */
  const [opening, setOpening] = useState<number | null>(null);

  const open = useCallback((at: number, go: () => Promise<NotOpened>) => {
    setOpening(at);
    setUnopened(null);
    void go()
      .then((why) => {
        // Nothing visible happens when a file opens behind the window, so the
        // silent case is the one that worked. The failure is the one that has
        // to speak, and it speaks on the row it was pressed on.
        if (why !== null) setUnopened({ row: at, because: why.because });
      })
      .finally(() => setOpening(null));
  }, []);

  return (
    <div className="armada-inside__where">
      {rows.map((row, at) => {
        const opens = row.open;
        const failed = unopened !== null && unopened.row === at ? unopened.because : null;
        return (
          <Fragment key={at}>
            <WhereRow
              label={row.iconLabel}
              value={row.value}
              note={row.meta}
              act={opens === undefined ? "copy" : "open"}
              copyValue={row.copyValue}
              onCopied={onCopied}
              actLabel={opens?.label}
              onAct={
                opens === undefined || opening !== null ? undefined : () => open(at, opens.go)
              }
            />
            {failed === null ? null : (
              <p className="armada-inside__where-unopened" role="status">
                {failed}
              </p>
            )}
          </Fragment>
        );
      })}
    </div>
  );
}
