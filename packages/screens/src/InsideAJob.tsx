// The arrangement `JobDetail` renders. Its rules are in
// `packages/components/src/screens/screens.css`, which Bridge and Storybook
// both load.

import type { ReactNode } from "react";
import { ChevronRight, ChevronUp } from "lucide-react";
import { Fragment, useCallback, useState } from "react";
import {
  RunTree,
  RunTreeSkeleton,
  Sheet,
  WhereRow,
  WorkflowDiagram,
  type JobBriefProps,
  type JobLogReferenceRow,
  type NotOpened,
  type RunTreeSkeletonProps,
  type RunTreeStep,
  type TaskMarkState,
} from "@armada/components";

import type { PlanEditAnswer } from "./plan-edits";
import { PlanPending, PlanWell } from "./PlanWell";
import { Inspector } from "./Inspector";
import type { StepOverview, StepPanel, StepReading } from "./Inspector";

// The inspector's own types are its file's. Re-exported here because a caller
// configures the arrangement, not the file a type sits in — `JobDetailProps`'
// own rule.
export { Eyebrow } from "./regions";
import { Eyebrow } from "./regions";
export type { StepNotice, StepOverview, StepPanel, StepReading } from "./Inspector";

/**
 * Inside a Job — the Overview tab, and one arrangement at every state.
 *
 * **The header and the tab strip are above this and are not its.** `#1534` gave
 * job detail five destinations and this is the first, unchanged; `JobDetail.tsx`
 * draws the strip, and what a tab holds is that tab's own file.
 *
 * **Under `--layout-breakpoint` the inspector folds to a sheet** — `narrow`,
 * below. Which regions there are and what order they read in do not move.
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

/** One task, as the Plan region draws it. `docs/concepts/plan.md`. */
export type PlanTaskRow = {
  id: string;
  title: string;
  state: TaskMarkState;
  /** Present on a dropped task, and on nothing else. */
  reason?: string;
  /** One line for what the other fields cannot hold. Absent where none. */
  note?: string;
  /** The paths the planner said this task touches. Absent where none. */
  scope?: readonly string[];
  /** What the planner said should prove it, and what the work said did. */
  expects?: string;
  shown?: string;
};

/**
 * The Job's plan, as the rail draws it between The run and Pulse. Absent
 * draws nothing — a Job whose workflow has no plan step, or one that has not
 * reached it yet.
 */
export type PlanRegionData = {
  approach: string;
  /** Every task, dropped included, in plan order. */
  tasks: readonly PlanTaskRow[];
};

/**
 * The Plan region's read of a Job: a plan recorded, or the step that will
 * record one where none exists yet. `docs/concepts/plan.md`; `#1007`.
 *
 * **Absent draws no region at all** — a workflow that declares no step
 * recording a plan. `recorded: false` is the state between that and a full
 * `PlanRegionData`: the step is declared, and its own label is what the
 * placeholder names.
 */
export type PlanRegionRead =
  | ({ recorded: true } & PlanRegionData)
  | { recorded: false; stepLabel: string };

export type InsideAJobProps = {
  /** The run, in order. One row per step of the frozen workflow. */
  run: RunTreeStep[];
  runLabel?: ReactNode;
  /**
   * The workflow's own name, beside the label — `Bug`. #1093 moved it here
   * from the header's facts, which already named the Job; this names the run.
   */
  runWorkflowLabel?: ReactNode;
  /** Why there is no run to draw, where there is none. */
  runAbsent?: string;
  /**
   * The run, while it is read: the workflow's step names, and nothing about
   * where any of them stands. Present takes the slot over `run`/`runAbsent`.
   */
  runReading?: RunTreeSkeletonProps;
  /**
   * The Jobs landing under this one, in the order they land — above the two
   * columns, because the order between them is what this Job *is* rather than
   * context for its own run. **Absent draws nothing**, which is every Job that
   * lands one pull request of its own. `#1543`.
   */
  members?: ReactNode;
  membersLabel?: ReactNode;
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
   * hole in the screen — the same rule `where` above keeps.
   */
  machine?: ReactNode;
  machineLabel?: ReactNode;
  /**
   * The control on the region's title line that opens the full reading. On the
   * title line and not in the block, where the run keeps its elapsed figure.
   */
  machineAct?: ReactNode;
  /**
   * The running mark on the current step animates. The tree names which step
   * is working, so it pulses and the header badge stays static.
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
   * The Job's plan, between The run and Pulse — shown whichever step is
   * selected, because the plan belongs to the Job and not to the step.
   * `recorded: false` draws the quiet placeholder naming the step that will
   * record it.
   */
  plan?: PlanRegionRead;
  /**
   * Add a task to the Job's plan, from the region's own eyebrow act — `Add
   * task`, the way Pulse carries `Details`. **Absent draws no act**, the same
   * condition the placeholder itself draws on: a Job whose workflow has no
   * plan step, or one whose plan step has not recorded one yet — `add_task`
   * is refused without a plan to add to. `jobId` is bound by the caller,
   * `JobDetail`'s own convention for every act drawn inside this screen.
   */
  onAddTask?: (title: string, detail: string) => Promise<PlanEditAnswer>;
  /**
   * Open one task's whole reading on the sheet layer. **The rail draws the
   * title, the state and a file count and nothing else** — a task's note, its
   * paths and both ends of its evidence are `#1421`'s fields and the rail is
   * 380px wide.
   */
  onOpenTask?: (taskId: string) => void;
  /**
   * Drop a task from the Job's plan, with a reason. Offered only on an `open`
   * or `working` row — `docs/concepts/plan.md`'s rule that a dropped task
   * stays dropped for a Drone, and a person's own add is what brings the work
   * back, as a new task.
   */
  onDropTask?: (taskId: string, reason: string) => Promise<PlanEditAnswer>;
  /**
   * A sentence the app is telling somebody, already written — `Added`,
   * `Dropped`. `onCopied`'s own shape: the row that changes is one among
   * several, so the act that changed it says so once, briefly, the way a
   * clipboard write does.
   */
  onSaid?: (sentence: string) => void;
  /**
   * Where things are — the worktree, the branch, the Manifest, the workflow,
   * the log, the transcript, the Drone. **A path opens where it lives; an
   * identifier copies.**
   */
  where?: JobLogReferenceRow[];
  whereLabel?: ReactNode;
  /** Why nothing can be named there, where nothing can. */
  whereAbsent?: string;
  /**
   * Whether Where things are is open. **Closed by default**, held by the
   * caller so it survives a live redraw — `detail-keys.ts` is where Bridge
   * already remembers this kind of choice. Absent draws it always open, which
   * is what a story with nothing to control wants.
   */
  whereOpen?: boolean;
  onOpenWhere?: (open: boolean) => void;
  /**
   * Everything the Job left behind, folded — its moves, its Drone's turns, what
   * it touched, what it changed, what it claimed.
   *
  /** The Job's brief, above the step on the panel's raised surface. */
  brief?: JobBriefProps;
  /** Why there is no brief, where there is none. */
  briefAbsent?: string;
  /** The brief is being read. Takes the slot over `brief`/`briefAbsent`. */
  briefLoading?: boolean;
  /** The step the panel is showing. */
  step?: StepPanel;
  /** Why no step is open, where none is. */
  stepAbsent?: string;
  /** The step, while it is read. Present takes the slot over `step`/`stepAbsent`. */
  stepReading?: StepReading;
  /**
   * The workflow overview, while a Job waits for approval. Its diagram takes
   * the run's place, and its facts take the panel over
   * `step`/`stepReading`/`stepAbsent` — the scope is the approval moment
   * alone, never the running Job's own step view.
   */
  overview?: StepOverview;
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
   * **An open sheet does not stop the pulse** (#1276). The tree's current step
   * is still working behind the layer, and the sheet's own live mark pulses
   * beside it.
   */
  sheet?: ReactNode;
  /**
   * The window is under `--layout-breakpoint`. **The inspector folds to a sheet
   * over the run**, which is the move Helm's dock already makes at the same
   * bound: two columns stop fitting, and a panel squeezed to a third of a
   * reading width is the v1 defect — a word a line — wearing this screen's
   * clothes.
   *
   * **A prop rather than a media query**, `Sheet.floor`'s own reason: a media
   * feature value cannot be a custom property, and the bound is a token.
   * `useNarrow` in `@armada/shell` is the one reader.
   */
  narrow?: boolean;
  /** The window is at `--window-floor`, where the folded inspector goes flush. */
  floor?: boolean;
  /**
   * Whether the folded inspector is open. Read only while `narrow` — above the
   * bound the inspector is a column and is always on screen.
   *
   * **Closed until a step is pressed.** A sheet that opened itself on arrival
   * would put a scrim over the run a reader came to read, and the run is what
   * says which step to open.
   */
  inspectorOpen?: boolean;
  /** The folded inspector's own name, which is the open step's. */
  inspectorTitle?: string;
  onCloseInspector?: () => void;
  onCopied?: (value: string) => void;
};

export function InsideAJob({
  run,
  runLabel = "The run",
  runWorkflowLabel,
  runAbsent = "Steps unknown",
  runReading,
  members,
  membersLabel = "Landing in order",
  machine,
  machineLabel = "Pulse",
  machineAct,
  pulsing = true,
  onSelectStep,
  openSteps,
  onOpenStep,
  onOpenArtifact,
  onOpenChapter,
  plan,
  onAddTask,
  onOpenTask,
  onDropTask,
  onSaid,
  where,
  whereLabel = "Where things are",
  whereAbsent = "Paths unknown",
  whereOpen,
  onOpenWhere,
  brief,
  briefAbsent = "No brief",
  briefLoading = false,
  step,
  stepAbsent = "Select a step in the run",
  stepReading,
  overview,
  unreachable,
  sheet,
  narrow = false,
  floor = false,
  inspectorOpen = false,
  inspectorTitle,
  onCloseInspector,
  onCopied,
}: InsideAJobProps) {
  // The inspector, built once and drawn in whichever of the two places the
  // window can pay for — a column beside the run, or a sheet over it.
  const panel = (
    <Inspector
      folded={narrow}
      {...(brief === undefined ? {} : { brief })}
      briefAbsent={briefAbsent}
      briefLoading={briefLoading}
      {...(step === undefined ? {} : { step })}
      stepAbsent={stepAbsent}
      {...(stepReading === undefined ? {} : { stepReading })}
      {...(overview === undefined ? {} : { overview })}
      {...(unreachable === undefined ? {} : { unreachable })}
    />
  );

  return (
    <>
      {/* Several pull requests landing in order, before the run: the order
          between the members is what a person opened this Job to read, and the
          parent's own four steps are how it got there. */}
      {members === undefined ? null : (
        <div className="armada-inside__landing">
          <div className="armada-inside__region-head">
            <Eyebrow>{membersLabel}</Eyebrow>
          </div>
          {members}
        </div>
      )}

      <div className="armada-inside" data-narrow={narrow || undefined}>
        {/* The run, and the pointers beneath it. Left, at every state. */}
        <div className="armada-inside__run">
          <div className="armada-inside__region-head">
            <Eyebrow>{runLabel}</Eyebrow>
            {runWorkflowLabel === undefined ? null : (
              <span className="armada-inside__region-meta">{runWorkflowLabel}</span>
            )}
          </div>
          {runReading !== undefined ? (
            <RunTreeSkeleton {...runReading} />
          ) : overview !== undefined ? (
            // Nothing has run yet, so the run is what the workflow declares.
            // It turns into the tree in this same place once the Job is approved.
            <WorkflowDiagram steps={overview.diagram} />
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

          {/* The plan, between the run and Pulse — the Job's own, not the
              step's, so it stays put whichever step is selected. Quiet
              placeholder before it is recorded, nothing where no step
              records one at all. */}
          {plan === undefined ? null : plan.recorded ? (
            <PlanWell
              approach={plan.approach}
              tasks={plan.tasks}
              onAddTask={onAddTask}
              onOpenTask={onOpenTask}
              onDropTask={onDropTask}
              onSaid={onSaid}
            />
          ) : (
            <PlanPending stepLabel={plan.stepLabel} />
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

          <WhereHead
            label={whereLabel}
            open={whereOpen ?? true}
            onOpen={onOpenWhere}
            branch={where?.find((row) => row.iconLabel === "Branch")?.value}
          />
          {(whereOpen ?? true) ? (
            where === undefined || where.length === 0 ? (
              <p className="armada-inside__absent" role="note">
                {unreachable ?? whereAbsent}
              </p>
            ) : (
              <WhereRegion rows={where} onCopied={onCopied} />
            )
          ) : null}
        </div>

        {/* The rule between the columns. Its own track, not a border on
            either side, so it measures the full height of the taller column
            whichever one that is. */}

        {/* The panel, beside the run wherever the window can pay for both.
            Same regions in the same order at every state. */}
        {narrow ? null : panel}
      </div>

      {/* Under `--layout-breakpoint` the inspector is a sheet over the run.
          **It is the screen's layer, not the window's** — `contained`, the
          same rule the log and the patch already keep, so the shell's rail
          stays out from under it.

          It gives the layer up the moment a reading takes it: one sheet at a
          time is this screen's rule, and two would answer one `Esc` between
          them — `inspectorOpen` is false while one is up, and closing the
          reading brings the inspector back where it was. */}
      {narrow ? (
        <Sheet
          open={inspectorOpen}
          contained
          floor={floor}
          title={inspectorTitle ?? "The step"}
          closeLabel="Close"
          closeBinding="Esc"
          bleed
          onClose={onCloseInspector}
        >
          {panel}
        </Sheet>
      ) : null}

      {sheet}
    </>
  );
}

/**
 * Where things are' own head — closed to one line by default, showing the
 * branch. `onOpen` absent draws a label rather than a control, `Eyebrow`'s
 * own rule for a region with nothing to toggle.
 */
function WhereHead({
  label,
  open,
  onOpen,
  branch,
}: {
  label: ReactNode;
  open: boolean;
  onOpen?: (open: boolean) => void;
  branch?: string;
}) {
  const text = (
    <span className="armada-inside__where-head-text">
      <Eyebrow>{label}</Eyebrow>
      {open || branch === undefined ? null : (
        <span className="armada-inside__where-branch">{branch}</span>
      )}
    </span>
  );
  if (onOpen === undefined) {
    return <div className="armada-inside__pulse-head">{text}</div>;
  }
  return (
    <button
      type="button"
      className="armada-inside__pulse-head armada-inside__where-toggle"
      aria-expanded={open}
      onClick={() => onOpen(!open)}
    >
      {text}
      {/* `Chapter`'s own pair: closed points at more to see, open points at
          closing it again — no rotated glyph, `docs/contracts/iconography.md`. */}
      {open ? (
        <ChevronUp size={12} strokeWidth={2} className="armada-inside__where-fold" aria-hidden />
      ) : (
        <ChevronRight size={12} strokeWidth={2} className="armada-inside__where-fold" aria-hidden />
      )}
    </button>
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
export function WhereRegion({
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
              // A *Serving* row names nothing that opens or copies on its
              // own — the one thing to press is `actions`, beside it — so a
              // row with neither `open` nor a value to copy draws no act at
              // all, rather than defaulting to a copy of nothing.
              act={opens !== undefined ? "open" : row.copyValue !== undefined ? "copy" : undefined}
              copyValue={row.copyValue}
              onCopied={onCopied}
              actLabel={opens?.label}
              onAct={
                opens === undefined || opening !== null ? undefined : () => open(at, opens.go)
              }
              run={row.run}
              actions={row.actions}
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
