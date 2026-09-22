// Workflow — the Job's run, drawn as the workflow it froze. `#1539`.
//
// **Canvas by default, stacked available, at every width** (#1530, 21 and 22
// Sep). The toggle is remembered per viewer, and where it is kept is the
// caller's: this package holds no storage.
//
// **Narrow opens on where you are.** A nine-step run fitted into 768px is nine
// cards nobody can read, so the canvas opens on the step a person is on and
// its neighbours instead.

import { Tabs, WorkflowCanvas, WorkflowInspector, WorkflowStacked } from "@armada/components";
import { useState } from "react";
import type { JobDetail as JobWhole, JobSummary } from "@armada/protocol";

import type { ConfirmableAct, HeldAct } from "./Acts";
import type { ActingAct } from "./pending";
import { TAB_LABEL } from "./detail-tabs";
import { taskGroupsOf } from "./draft/group";
import { ACT_LABEL, HOLD_LABEL, HOLD_SAID } from "./copy";
import { steeringOf } from "./steering";
import { stepTheGroupsHangUnder, workflowRunOf } from "./workflow-canvas";
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
  const [instruction, setInstruction] = useState("");

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

  const groups = taskGroupsOf(whole);
  const groupsUnder = stepTheGroupsHangUnder(whole);
  const run = workflowRunOf({ whole, groups, onOpen: setOpen });
  const reading = workflowReadingOf({ whole, groups, selected: open, groupsUnder });
  const steering = steeringOf(job, whole);

  return (
    <div className="armada-detail-tab" role="tabpanel" aria-label={TAB_LABEL.workflow}>
      <div className="armada-workflow-tab" data-narrow={narrow || undefined}>
        <div className="armada-workflow-tab__surface">
          {view === "canvas" ? (
            <div className="armada-workflow-tab__canvas">
              <WorkflowCanvas
                nodes={run.nodes}
                edges={run.edges}
                label={`${job.title}, as its workflow's steps`}
                running={run.running}
                {...(narrow ? { opensOn: run.opensOn } : {})}
                aside={toggle}
              />
            </div>
          ) : (
            <WorkflowStacked label={`${job.title}, as its workflow's steps`} rows={run.rows} aside={toggle} />
          )}
        </div>

        <div className="armada-workflow-tab__inspector">
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
