import { ChevronDown, ChevronRight } from "lucide-react";

import { FactChip } from "../FactChip/FactChip";
import { GroupBoundary, type GroupBoundaryProps } from "../GroupBoundary/GroupBoundary";
import { GuideMark } from "../GuideMark/GuideMark";
import { GUIDE_GROUP_ORDER } from "../../guides";
import { StepBar } from "../StepBar/StepBar";
import { TaskMark, type TaskMarkState } from "../TaskMark/TaskMark";
import type { PlanGroupState } from "../PlanBoard/PlanBoard";

/**
 * The implement step, opened — its groups, each opening into its tasks, and
 * what ran at each group's end. `#1536`.
 *
 * **This is the run, not the plan.** `PlanBoard` draws the split a person
 * approves before anything moves; this draws which task is moving, what it has
 * cost since its own agent stopped, and which Check broke the boundary.
 *
 * **A group opens; it is not always open.** Eight tasks and four boundaries
 * drawn at once is the wall of rows the issue was filed against, so the group
 * that is moving opens itself and the rest are one line each.
 *
 * **It composes no sentence.** Every line of prose is the caller's.
 */

/** One task's row, inside its group. */
export type ImplementTask = {
  /** `T1`, `T2`, … as the plan numbered it. */
  id: string;
  title: string;
  mark: TaskMarkState;
  /** `difficult · opus · its own agent` — the tier, its model and how it is run. */
  says: string;
  /**
   * `14 turns`, or `34 turns · ~$2.40` once its own agent stopped. **Never a
   * live cost** (`#1530`, 22 Sep): cost reaches Armada on a session's last line.
   */
  spentSays?: string;
  /** `runs beside T6`. Absent where the task runs alone. */
  besideSays?: string;
  /** `touched later · T7` — a later task edited a file this one had finished. */
  touchedSays?: string;
  /** Why its own agent stopped. Present on a failed task and on nothing else. */
  failedReason?: string;
};

export type ImplementGroup = {
  id: string;
  /** Its position in the step, counted from one. */
  ordinal: number;
  state: PlanGroupState;
  /** Where the group is, in words — `working`, `failed at its checks`. */
  says: string;
  /**
   * `2 tasks, at the same time` or `2 tasks, one after another`. **Fan out,
   * then join**: the shape is what the line says, and the bar under it is what
   * the group has got through.
   */
  shapeSays: string;
  /**
   * The commit it left. **On the closed line as well as at the boundary**: a
   * group folded shut still has to say what it got through, or reading a run is
   * opening every group in turn.
   */
  commit?: string;
  tasks: readonly ImplementTask[];
  boundary: GroupBoundaryProps;
};

export type ImplementBoardProps = {
  /** The step's own label — `Implement`, off the frozen workflow. */
  stepName: string;
  /** One line above the groups: one group at a time, and what that forbids. */
  orderSays: string;
  groups: readonly ImplementGroup[];
  /** Which groups are open, by id. */
  openGroups: readonly string[];
  onOpenGroup: (groupId: string) => void;
  /** The task the inspector is on, so its row says so. */
  openTaskId?: string;
  onOpenTask: (taskId: string) => void;
};

/** A task's mark, on the boundary bar's segment grammar. */
function segmentOf(mark: TaskMarkState) {
  if (mark === "done") return "done" as const;
  if (mark === "working") return "working" as const;
  if (mark === "failed") return "failed" as const;
  return "open" as const;
}

function TaskRow({
  task,
  open,
  onOpenTask,
}: {
  task: ImplementTask;
  open: boolean;
  onOpenTask: (taskId: string) => void;
}) {
  return (
    <li className="armada-implement__task" aria-label={`${task.id} ${task.title}`} data-mark={task.mark}>
      <button
        type="button"
        className="armada-implement__task-head"
        aria-current={open ? "true" : undefined}
        onClick={() => onOpenTask(task.id)}
      >
        <TaskMark state={task.mark} />
        <span className="armada-implement__task-id mono">{task.id}</span>
        <span className="armada-implement__task-title">{task.title}</span>
        <span className="armada-implement__task-says">{task.says}</span>
        {task.spentSays === undefined ? null : <FactChip>{task.spentSays}</FactChip>}
      </button>
      {task.besideSays === undefined && task.touchedSays === undefined ? null : (
        <p className="armada-implement__task-notes">
          {task.besideSays === undefined ? null : (
            <span className="armada-implement__task-beside">{task.besideSays}</span>
          )}
          {task.touchedSays === undefined ? null : (
            <span className="armada-implement__task-touched">{task.touchedSays}</span>
          )}
        </p>
      )}
      {task.failedReason === undefined ? null : (
        <p className="armada-implement__task-failed">{task.failedReason}</p>
      )}
    </li>
  );
}

function Group({
  group,
  open,
  openTaskId,
  onOpenGroup,
  onOpenTask,
}: {
  group: ImplementGroup;
  open: boolean;
  openTaskId?: string;
  onOpenGroup: (groupId: string) => void;
  onOpenTask: (taskId: string) => void;
}) {
  const name = `Group ${group.ordinal}`;
  const Chevron = open ? ChevronDown : ChevronRight;
  return (
    <li className="armada-implement__group" data-state={group.state}>
      <button
        type="button"
        className="armada-implement__group-head"
        aria-expanded={open}
        onClick={() => onOpenGroup(group.id)}
      >
        <Chevron className="armada-implement__fold" size={13} strokeWidth={2} aria-hidden />
        <h3 className="armada-implement__group-name">{name}</h3>
        <span className="armada-implement__state" data-state={group.state}>
          {group.says}
        </span>
        <span className="armada-implement__shape">{group.shapeSays}</span>
        {group.commit === undefined ? null : <FactChip title={group.commit}>{group.commit}</FactChip>}
        <StepBar
          tasks={group.tasks.map((task) => segmentOf(task.mark))}
          label={`${name}, ${group.shapeSays}`}
        />
      </button>
      {/* Hidden rather than unmounted: a boundary's output scrolled to stays
          where it was when a reader folds the group and opens it again. */}
      <div className="armada-implement__body" hidden={!open}>
        <ul className="armada-implement__tasks" aria-label={`${name} tasks`}>
          {group.tasks.map((task) => (
            <TaskRow
              key={task.id}
              task={task}
              open={task.id === openTaskId}
              onOpenTask={onOpenTask}
            />
          ))}
        </ul>
        <GroupBoundary {...group.boundary} />
      </div>
    </li>
  );
}

export function ImplementBoard({
  stepName,
  orderSays,
  groups,
  openGroups,
  onOpenGroup,
  openTaskId,
  onOpenTask,
}: ImplementBoardProps) {
  return (
    <section className="armada-implement" aria-label={`${stepName}, opened`}>
      {/* The mark goes on the order line, not on a group head — that head is a
          button, and a button inside a button is not markup a browser keeps.
          What it explains is the rule the line is the only evidence of. */}
      <p className="armada-implement__order" role="note">
        {orderSays}
        <GuideMark guide={GUIDE_GROUP_ORDER} />
      </p>
      <ol className="armada-implement__groups" aria-label="The groups of this step, in the order they run">
        {groups.map((group) => (
          <Group
            key={group.id}
            group={group}
            open={openGroups.includes(group.id)}
            {...(openTaskId === undefined ? {} : { openTaskId })}
            onOpenGroup={onOpenGroup}
            onOpenTask={onOpenTask}
          />
        ))}
      </ol>
    </section>
  );
}
