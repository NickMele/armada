// Workflow — the Job's run, drawn as the workflow it froze. `#1539`.
//
// **A full canvas, not a spine** (owner, 23 Sep 2026): the steps, the groups
// hanging off the step that wrote them, the tasks hanging off their groups,
// and a second edge from the step that worked a group. The running step's
// board stays under it — a node says where a group came from, the board says
// what happened inside it, and neither says the other.
//
// **Canvas by default, stacked available, at every width** (#1530, 21 and 22
// Sep). The toggle is remembered per viewer, and where it is kept is the
// caller's: this package holds no storage.
//
// **Narrow opens on where you are.** A whole plan fitted into 768px is cards
// nobody can read, so the canvas narrows onto the step a person is on and the
// plan that step holds rather than shrinking the run.

import {
  ImplementBoard,
  Tabs,
  WorkflowCanvas,
  WorkflowInspector,
  WorkflowStacked,
} from "@armada/components";
import { useState } from "react";
import type { JobDetail as JobWhole, JobSummary, Turn } from "@armada/protocol";

import type { ConfirmableAct, HeldAct } from "./Acts";
import type { ActingAct } from "./pending";
import type { CaseView } from "./draft/cases";
import { TAB_LABEL } from "./detail-tabs";
import { taskGroupsOf, type GroupView } from "./draft/group";
import { ACT_LABEL, HOLD_LABEL, HOLD_SAID } from "./copy";
import { groupsThatOpen, implementBoardOf } from "./implement";
import { taskReadingOf } from "./implement-task";
import { steeringOf } from "./steering";
import {
  groupNodeId,
  stepThatWorksTheGroups,
  taskNodeId,
  taskOfNodeId,
  workflowRunOf,
} from "./workflow-canvas";
import { workflowReadingOf } from "./workflow-inspector";
import { WORKFLOW_VIEWS, WORKFLOW_VIEW_LABEL, type WorkflowView } from "./workflow-view";

export type WorkflowTabProps = {
  job: JobSummary;
  /** The Job whole. `null` while the read is in flight, or where it failed. */
  whole: JobWhole | null;
  /** Why there is no run to draw, where there is none. */
  absent?: string;
  /**
   * Whether the window is narrow enough that the inspector takes the whole
   * width of the tab when it opens, rather than a panel's width over one side
   * of the canvas. `useNarrow`'s answer.
   */
  narrow: boolean;
  view: WorkflowView;
  onView: (view: WorkflowView) => void;
  /**
   * The groups the implement step opens into, where this Job's boards were
   * handed them (`#1532`'s draft, on `JobDetailProps.draft`). **Absent derives
   * one group per task from what Fleet serves**, which is thinner and never
   * broken — `draft/group.ts` carries the reasoning.
   */
  groups?: readonly GroupView[];
  /** The cases the plan owes, for the tests drawn apart at each boundary. */
  cases?: readonly CaseView[];
  /** How many Drones this Job may run at once, as the gate settled it. `#1550`. */
  droneCap?: number;
  /**
   * The Job's turns, where the second socket is carrying them. A task's own
   * lines and its last edit are read off these — `implement-task.ts`.
   */
  turns?: readonly Turn[];
  /** Whether anything is watching those turns, so an absence can say which it is. */
  watching?: boolean;
  /** A press is out and Fleet has not answered. */
  acting: boolean;
  actingAct?: ActingAct;
  onRedirect: (jobId: string, instruction: string) => void;
  onAct: (act: ConfirmableAct, jobId: string) => void;
  onActHeld: (act: HeldAct, jobId: string) => void;
};

export function WorkflowTab({
  job,
  whole,
  absent,
  narrow,
  view,
  onView,
  groups: given,
  cases = [],
  droneCap,
  turns = [],
  watching = false,
  acting,
  actingAct,
  onRedirect,
  onAct,
  onActHeld,
}: WorkflowTabProps) {
  // The node a person has open. **Not the running step held in state** — that
  // moves under them as the Job advances, and a panel that changed subject
  // while somebody was reading it is the surface this screen exists to escape.
  const [open, setOpen] = useState<string | null>(null);
  // Whether the canvas re-centres on the running step as the Job advances.
  // **Off until it is asked for**: it wins over the fit, and a run opened
  // centred on one card is a run with its other steps off screen.
  const [following, setFollowing] = useState(false);
  const [instruction, setInstruction] = useState("");
  // The task a person has open, and which groups they have folded or unfolded.
  // **`null` for the groups means nobody has chosen yet**, so the group that is
  // moving opens itself — and a person's first press is what takes that over.
  const [openTask, setOpenTask] = useState<string | null>(null);
  const [opened, setOpened] = useState<string[] | null>(null);

  // **Nothing scrolls the panel into view any more, because it is never out of
  // it.** Until 25 Sep the panel was a column that folded under the whole graph
  // at a narrow window, so a press on a task moved a reply box nobody could
  // see and an effect wrote scroll position to fix it. The panel is a layer
  // over the canvas now and sticks to the top of the destination
  // (`screens.css`), so review and reply stay one loop with no scrolling at all.

  const toggle = (
    <Tabs
      items={WORKFLOW_VIEWS.map((one) => ({ id: one, label: WORKFLOW_VIEW_LABEL[one] }))}
      value={view}
      onChange={(id) => onView(id as WorkflowView)}
    />
  );

  if (whole === null || whole.steps.length === 0) {
    return (
      <div className="armada-detail-tab" role="tabpanel" aria-label={TAB_LABEL.workflow}>
        <p className="armada-inside__absent" role="note">
          {absent ?? "This Job's frozen workflow has no steps."}
        </p>
      </div>
    );
  }

  const groups = given ?? taskGroupsOf(whole);
  const groupsUnder = stepThatWorksTheGroups(whole);
  const step = whole.steps.find((one) => one.step_id === groupsUnder);
  const openGroups = opened ?? groupsThatOpen(groups);
  // The whole plan, as one graph. A press on a task card opens that task and
  // presses it again to close; a press on a step or a group takes the panel
  // off whichever task was open rather than leaving two selections live.
  const run = workflowRunOf({
    whole,
    groups,
    selected: openTask === null ? open : taskNodeId(openTask),
    onOpen: (id) => {
      const task = taskOfNodeId(id);
      if (task !== undefined) {
        setOpenTask(task === openTask ? null : task);
        return;
      }
      setOpen(id);
      setOpenTask(null);
    },
  });
  // The running step, opened under the graph — `#1536`. **The graph says where
  // a group came from; the board says what happened inside it**: the commit it
  // left, what its boundary came to, what stopping it holds back and the failed
  // Check's own output handed to the next Drone. A node on a canvas carries
  // none of that, which is why the two are not the same thing drawn twice.
  const board = implementBoardOf({
    whole,
    groups,
    cases,
    step,
    ...(droneCap === undefined ? {} : { droneCap }),
    openGroups,
    onOpenGroup: (id) => {
      setOpened(openGroups.includes(id) ? openGroups.filter((one) => one !== id) : [...openGroups, id]);
      setOpen(groupNodeId(id));
      setOpenTask(null);
    },
    ...(openTask === null ? {} : { openTaskId: openTask }),
    onOpenTask: (id) => setOpenTask(id === openTask ? null : id),
  });
  // **Nothing is open until a press opens it** (owner, 25 Sep 2026). The panel
  // used to land on the step the Job is on, so the column beside the canvas was
  // never blank — there is no column now. The canvas has the tab's whole width
  // and this is a layer over it, so a reading nobody asked for would be a panel
  // covering the run it exists to explain. A task outranks a node: it is the
  // newest press and the narrowest reading.
  const reading =
    (openTask === null
      ? undefined
      : taskReadingOf({
          whole,
          groups,
          taskId: openTask,
          turns,
          watching,
          ...(groupsUnder === undefined ? {} : { stepId: groupsUnder }),
        })) ?? workflowReadingOf({ whole, groups, selected: open, groupsUnder });
  const steering = steeringOf(job, whole);
  const label = `${job.title}, as its workflow's run`;
  // Whether anything hangs off the run. A spine wants a shorter frame than a
  // run carrying three depths, and a taller one around it would be void.
  const plan = run.rows.some((row) => row.depth !== undefined);

  return (
    <div className="armada-detail-tab" role="tabpanel" aria-label={TAB_LABEL.workflow}>
      <div
        className="armada-workflow-tab"
        data-view={view}
        data-narrow={narrow || undefined}
        data-plan={plan || undefined}
      >
        <div className="armada-workflow-tab__surface">
          {/* Above the run rather than over it: drawn inside the canvas the
              toggle sat on top of the last step's card at every width.

              No `?` here any more. Guide 11 explained how a group gets its
              second edge; the plan's graph moved to the Plan tab on 25
              September 2026, so the guide was retired and its number with it. */}
          <div className="armada-workflow-tab__modes">{toggle}</div>
          {view === "canvas" ? (
            <div className="armada-workflow-tab__canvas">
              <WorkflowCanvas
                nodes={run.nodes}
                edges={run.edges}
                label={label}
                running={run.running}
                following={following}
                onFollowing={setFollowing}
                opensOn={run.opensOn}
              />
            </div>
          ) : (
            <WorkflowStacked label={label} rows={run.rows} />
          )}
          {/* What is inside the step the Job is on. The graph above says where
              each group came from; this says what happened in it. `#1536`. */}
          {board === undefined ? null : (
            <div className="armada-workflow-tab__opened">
              <ImplementBoard {...board} />
            </div>
          )}
        </div>

        {/* Nothing until a press, and then a layer over the canvas rather than
            a column beside it — the dock's own arrangement, `screens.css`. The
            canvas keeps the tab's width either way. */}
        {reading === undefined ? null : (
          <div className="armada-workflow-tab__inspector-layer">
            <div className="armada-workflow-tab__inspector">
              <WorkflowInspector
                {...reading}
                onClose={() => {
                  setOpen(null);
                  setOpenTask(null);
                }}
                redirect={{
                  value: instruction,
                  onChange: setInstruction,
                  onSend: () => {
                    onRedirect(job.id, instruction);
                    setInstruction("");
                  },
                  drones: reading.drones,
                  disabled: steering.act === undefined,
                  disabledReason: NO_DRONE,
                  ...(steering.sent === undefined ? {} : { waiting: steering.sent }),
                }}
                {...(steering.act === undefined
                  ? {}
                  : {
                      stop: {
                        children: HOLD_LABEL.kill_drone,
                        askLabel: ACT_LABEL.kill_drone,
                        description: HOLD_SAID.kill_drone,
                        disabled: acting && actingAct !== "kill_drone",
                        pending: acting && actingAct === "kill_drone",
                        onAsk: () => onAct("kill_drone", job.id),
                        onCommit: () => onActHeld("kill_drone", job.id),
                      },
                    })}
              />
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

/**
 * Why the box is closed. **One Drone per Job today**, so a step with nothing on
 * it has nothing to reach — and stopping a group is stopping that Drone until
 * Fleet runs one per task.
 */
const NO_DRONE = "No Drone is on this Job, so there is nothing to redirect.";
