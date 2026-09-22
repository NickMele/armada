import { ChevronRight } from "lucide-react";
import { Card, CardContent, CardHeader, CardTitle } from "../../primitives/Card/Card";
import { Clamped } from "../Clamped/Clamped";
import { FactChip } from "../FactChip/FactChip";
import { PathChip } from "../PathChip/PathChip";
import { TaskMark, type TaskMarkState } from "../TaskMark/TaskMark";

/**
 * Plan board — a plan read as the groups it will run in, one card each.
 *
 * **The card is the group, because the group is where the Checks run**
 * (`#1530`, 21 Sep). A task carries what its own agent does; the boundary
 * under it carries what runs once every task in the card has stopped.
 *
 * **It composes no sentence.** Every line of prose here is written by the
 * caller — `packages/screens/src/tab-plan-read.ts` — for `StepBar`'s own
 * reason: a count in words is copy, and copy has one owner.
 */

/** Where a group is, for its hue alone. The word is `says`, which is prose. */
export type PlanGroupState =
  | "pending"
  | "running"
  | "joining"
  | "checking"
  | "passed"
  | "failed"
  | "retrying"
  | "landed";

/** One task's row. `mark` is the whole of its state — no row prints a word. */
export type PlanBoardTask = {
  /** `T1`, `T2`, … as the plan numbered it. */
  id: string;
  title: string;
  mark: TaskMarkState;
  /** How hard the planner thought it was, and the model that tier resolved to. */
  tier: string;
  model: string;
  /** `runs beside T5`, written by the caller. Absent where it runs alone. */
  besideSays?: string;
  /** `touched later · T7` — a later task edited a file this one had finished. */
  touchedSays?: string;
  /** Why its agent stopped. Present on a failed task and on nothing else. */
  failedReason?: string;
  /** `34 turns · $2.40`, or `14 turns` while it is still working. */
  spentSays?: string;
};

/**
 * One case at a boundary or on a task. **`reads` is the whole claim**: a case
 * with no spec reads `not covered` and never green.
 */
export type PlanBoardTest = {
  id: string;
  /** The spec's repository path, or the case's own id where it has no spec. */
  spec: string;
  reads: "owed" | "not covered" | "dropped";
  /** What dropped it. Present on `dropped` and on nothing else. */
  droppedSays?: string;
};

export type PlanBoardGroup = {
  id: string;
  /** Its position in the step, counted from one. */
  ordinal: number;
  state: PlanGroupState;
  /** Where the group is, in words — `working`, `passed`, `failed at its checks`. */
  says: string;
  /** Every path its tasks claim, de-duplicated, in first-seen order. */
  scope: readonly string[];
  tasks: readonly PlanBoardTask[];
  /** `7 checks will run at this boundary`. */
  boundarySays: string;
  /** The Checks by name, in the order the Manifest declares them. */
  checks: readonly string[];
  /** Which of them failed, by name. Drawn beside the ones that passed. */
  checksFailed?: readonly string[];
  /** `1 test runs at this boundary`. Absent where none does. */
  testsSay?: string;
  tests?: readonly PlanBoardTest[];
  /** `second run`, where the group has been run again. */
  retrySays?: string;
  /** The commit it left. Absent until it left one. */
  commit?: string;
  /** `both tasks run at the same time`. Absent where they run in order. */
  concurrentSays?: string;
};

/** A file two groups both claim, and which tasks claim it. */
export type PlanBoardClash = { path: string; says: string };

export type PlanBoardProps = {
  /** The plan's own approach line, as the step recorded it. */
  approach: string;
  /** How the groups run — one line, above the cards. */
  orderSays: string;
  groups: readonly PlanBoardGroup[];
  /** Where the order contradicts the scopes. Empty draws nothing. */
  clashes?: readonly PlanBoardClash[];
  /** The task the inspector is open on, so the row it came from says so. */
  openTaskId?: string;
  onOpenTask?: (taskId: string) => void;
};

/** The directory half of a path, trailing separator kept — `PathChip`'s rule. */
function splitPath(path: string): { directory?: string; basename: string } {
  const cut = path.lastIndexOf("/");
  if (cut < 0) return { basename: path };
  return { directory: path.slice(0, cut + 1), basename: path.slice(cut + 1) };
}

function TaskRow({
  task,
  open,
  onOpenTask,
}: {
  task: PlanBoardTask;
  open: boolean;
  onOpenTask?: (taskId: string) => void;
}) {
  const body = (
    <>
      <TaskMark state={task.mark} />
      <span className="armada-plan-board__task-id">{task.id}</span>
      <span className="armada-plan-board__task-title">{task.title}</span>
      <span className="armada-plan-board__task-tier">
        {task.tier} · {task.model}
      </span>
    </>
  );
  return (
    // **Named by its task, because the mark speaks first.** `TaskMark` carries
    // its state as the row's leading text for a screen reader, so a row with no
    // name of its own is announced "Open T6 …" and cannot be reached by the id
    // a plan is discussed in.
    <li className="armada-plan-board__task" data-mark={task.mark} aria-label={`${task.id} ${task.title}`}>
      {onOpenTask === undefined ? (
        <span className="armada-plan-board__task-head">{body}</span>
      ) : (
        <button
          type="button"
          className="armada-plan-board__task-head"
          aria-current={open ? "true" : undefined}
          onClick={() => onOpenTask(task.id)}
        >
          {body}
          <ChevronRight size={12} strokeWidth={2} aria-hidden />
        </button>
      )}
      {task.besideSays === undefined &&
      task.touchedSays === undefined &&
      task.spentSays === undefined ? null : (
        <span className="armada-plan-board__task-notes">
          {task.spentSays === undefined ? null : <FactChip>{task.spentSays}</FactChip>}
          {task.besideSays === undefined ? null : (
            <span className="armada-plan-board__task-beside">{task.besideSays}</span>
          )}
          {/* The flag, and the whole of what #1530 decided: the task stays done
              and says which later one reached into its files. */}
          {task.touchedSays === undefined ? null : (
            <span className="armada-plan-board__task-touched">{task.touchedSays}</span>
          )}
        </span>
      )}
      {task.failedReason === undefined ? null : (
        <span className="armada-plan-board__task-failed">{task.failedReason}</span>
      )}
    </li>
  );
}

function Boundary({ group }: { group: PlanBoardGroup }) {
  const failed = new Set(group.checksFailed ?? []);
  return (
    <div className="armada-plan-board__boundary">
      <p className="armada-plan-board__boundary-says">{group.boundarySays}</p>
      {/* Every Check, named, including the ones that passed. A group reading
          red with nothing said is what the issue asked to be rid of. */}
      <ul className="armada-plan-board__checks">
        {group.checks.map((name) => (
          <li key={name}>
            <FactChip {...(failed.has(name) ? { named: "failed" as const } : {})}>{name}</FactChip>
          </li>
        ))}
      </ul>
      {group.testsSay === undefined ? null : (
        <p className="armada-plan-board__boundary-says">{group.testsSay}</p>
      )}
      {group.tests === undefined || group.tests.length === 0 ? null : (
        <ul className="armada-plan-board__tests">
          {group.tests.map((test) => (
            <li key={test.id}>
              {/* **Only the exceptional reading is noted.** A note on every
                  row would put a word beside each and say nothing; the row
                  worth stopping on is the case nothing will run. */}
              <PathChip
                {...splitPath(test.spec)}
                {...(test.reads === "owed"
                  ? {}
                  : { note: test.droppedSays ?? test.reads })}
              />
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

function GroupCard({
  group,
  openTaskId,
  onOpenTask,
}: {
  group: PlanBoardGroup;
  openTaskId?: string;
  onOpenTask?: (taskId: string) => void;
}) {
  return (
    <li className="armada-plan-board__group">
      <Card data-state={group.state}>
        <CardHeader>
          <CardTitle>Group {group.ordinal}</CardTitle>
          <span className="armada-plan-board__state" data-state={group.state}>
            {group.says}
          </span>
        </CardHeader>
        <CardContent>
          <div className="armada-plan-board__group-facts">
            {group.concurrentSays === undefined ? null : (
              <FactChip>{group.concurrentSays}</FactChip>
            )}
            {group.retrySays === undefined ? null : <FactChip>{group.retrySays}</FactChip>}
            {group.commit === undefined ? null : <FactChip>{group.commit}</FactChip>}
          </div>
          <ul className="armada-plan-board__scope" aria-label={`Group ${group.ordinal} scope`}>
            {group.scope.map((path) => (
              <li key={path}>
                <PathChip {...splitPath(path)} />
              </li>
            ))}
          </ul>
          <ul className="armada-plan-board__tasks" aria-label={`Group ${group.ordinal} tasks`}>
            {group.tasks.map((task) => (
              <TaskRow
                key={task.id}
                task={task}
                open={task.id === openTaskId}
                {...(onOpenTask === undefined ? {} : { onOpenTask })}
              />
            ))}
          </ul>
          <Boundary group={group} />
        </CardContent>
      </Card>
    </li>
  );
}

export function PlanBoard({
  approach,
  orderSays,
  groups,
  clashes = [],
  openTaskId,
  onOpenTask,
}: PlanBoardProps) {
  return (
    <div className="armada-plan-board">
      <section className="armada-plan-board__approach" aria-label="The approach">
        <Clamped lines={3}>{approach}</Clamped>
        <p className="armada-plan-board__order" role="note">
          {orderSays}
        </p>
      </section>
      {clashes.length === 0 ? null : (
        <section className="armada-plan-board__clashes" aria-label="Two groups claim the same file">
          <h3 className="armada-plan-board__clashes-title">Two groups claim the same file</h3>
          <ul>
            {clashes.map((clash) => (
              <li key={clash.path}>
                <PathChip {...splitPath(clash.path)} note={clash.says} />
              </li>
            ))}
          </ul>
        </section>
      )}
      <ol className="armada-plan-board__groups" aria-label="Groups, in the order they run">
        {groups.map((group) => (
          <GroupCard
            key={group.id}
            group={group}
            {...(openTaskId === undefined ? {} : { openTaskId })}
            {...(onOpenTask === undefined ? {} : { onOpenTask })}
          />
        ))}
      </ol>
    </div>
  );
}
