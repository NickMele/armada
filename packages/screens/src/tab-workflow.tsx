// Workflow — the Job's run, drawn as the workflow it froze. `#1539`.
//
// **Canvas by default, stacked available, at every width** (#1530, 21 and 22
// Sep). The toggle is remembered per viewer, and where it is kept is the
// caller's: this package holds no storage.
//
// **Narrow opens on where you are.** A nine-step run fitted into 768px is nine
// cards nobody can read, so the canvas opens on the step a person is on and
// its neighbours instead.

import { ImplementBoard, Tabs, WorkflowCanvas, WorkflowInspector, WorkflowStacked } from "@armada/components";
import { useEffect, useRef, useState } from "react";
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
import { groupNodeId, stepTheGroupsHangUnder, workflowRunOf } from "./workflow-canvas";
import { workflowReadingOf } from "./workflow-inspector";
import { WORKFLOW_VIEWS, WORKFLOW_VIEW_LABEL, type WorkflowView } from "./workflow-view";

export type WorkflowTabProps = {
  job: JobSummary;
  /** The Job whole. `null` while the read is in flight, or where it failed. */
  whole: JobWhole | null;
  /** Why there is no run to draw, where there is none. */
  absent?: string;
  /** Whether the inspector has a column of its own. `useNarrow`'s answer. */
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

  // The panel, so a press on a task can bring it into view. **Review and reply
  // are one loop**: folded to one column the panel sits under the whole board,
  // and a press that moved a reply box nobody can see is two surfaces. An
  // effect because what it writes is scroll position, which is outside React.
  const panel = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (openTask === null) return;
    const box = panel.current?.getBoundingClientRect();
    if (box === undefined || (box.top >= 0 && box.top < window.innerHeight)) return;
    panel.current?.scrollIntoView({ block: "nearest" });
  }, [openTask]);

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
  const groupsUnder = stepTheGroupsHangUnder(whole);
  const step = whole.steps.find((one) => one.step_id === groupsUnder);
  const openGroups = opened ?? groupsThatOpen(groups);
  // The implement step, opened under the run — `#1536`. The canvas above is
  // where the step sits; this is what is inside it.
  const board = implementBoardOf({
    whole,
    groups,
    cases,
    step,
    openGroups,
    onOpenGroup: (id) => {
      setOpened(openGroups.includes(id) ? openGroups.filter((one) => one !== id) : [...openGroups, id]);
      setOpen(groupNodeId(id));
      setOpenTask(null);
    },
    ...(openTask === null ? {} : { openTaskId: openTask }),
    onOpenTask: (id) => setOpenTask(id === openTask ? null : id),
  });
  // The run above the board. **Its groups hang on it only where nothing has
  // opened them below** — the same groups drawn twice is what made the canvas
  // too tall to fit once it was drawn small.
  //
  // A press on a card is a press on a step or a group, so it takes the panel
  // off whichever task was open rather than leaving two selections live.
  const run = workflowRunOf({
    whole,
    groups,
    opened: board !== undefined,
    onOpen: (id) => {
      setOpen(id);
      setOpenTask(null);
    },
  });
  // **The inspector lands on the step the Job is on**, so the panel is never a
  // blank column beside a full canvas. A person's own press then wins, and the
  // Job advancing does not take the panel off what they are reading. A task
  // outranks both: it is the newest press and the narrowest reading.
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
        })) ?? workflowReadingOf({ whole, groups, selected: open ?? run.running, groupsUnder });
  const steering = steeringOf(job, whole);

  return (
    <div className="armada-detail-tab" role="tabpanel" aria-label={TAB_LABEL.workflow}>
      <div
        className="armada-workflow-tab"
        data-view={view}
        data-narrow={narrow || undefined}
        data-opened={board === undefined ? undefined : true}
      >
        <div className="armada-workflow-tab__surface">
          {/* Above the run rather than over it: drawn inside the canvas the
              toggle sat on top of the last step's card at every width. */}
          <div className="armada-workflow-tab__modes">{toggle}</div>
          {view === "canvas" ? (
            <div className="armada-workflow-tab__canvas">
              <WorkflowCanvas
                nodes={run.nodes}
                edges={run.edges}
                label={`${job.title}, as its workflow's steps`}
                running={run.running}
                following={following}
                onFollowing={setFollowing}
                opensOn={run.opensOn}
              />
            </div>
          ) : (
            <WorkflowStacked label={`${job.title}, as its workflow's steps`} rows={run.rows} />
          )}
          {/* The step the canvas draws as one card, opened. The run above is
              the same canvas the destination has always used, drawn small so
              what is inside the step has the room. `#1536`. */}
          {board === undefined ? null : (
            <div className="armada-workflow-tab__opened">
              <ImplementBoard {...board} />
            </div>
          )}
        </div>

        <div className="armada-workflow-tab__inspector" ref={panel}>
          {reading === undefined ? (
            <p className="armada-inside__absent" role="note">
              Press a step or a group to read what it is doing.
            </p>
          ) : (
            <WorkflowInspector
              {...reading}
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
          )}
        </div>
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
