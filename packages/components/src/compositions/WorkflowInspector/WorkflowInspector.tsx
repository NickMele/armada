import { DroneMessageBox, type DroneMessageBoxProps } from "../DroneMessageBox/DroneMessageBox";
import { FactChip, type FactChipNamed } from "../FactChip/FactChip";
import { HoldButton, type HoldButtonProps } from "../../primitives/HoldButton/HoldButton";
import { Select } from "../../primitives/Select/Select";

/**
 * One step or group of a Job's workflow, read whole — what it is doing, the
 * tasks under it, the Checks at its boundary, the tests at that same boundary,
 * and the two things a person may do about it. `#1539`.
 *
 * **Tests are drawn apart from the Checks** (`#1530`, 22 Sep). A Check is the
 * repository's command; a case is what the work owes. Folding them into one
 * list would make a case with no spec read as a Check that passed.
 *
 * **Review and reply are one loop**, so the redirect is in this panel and not
 * behind a dialog somewhere else — and it says which Drone it reaches, because
 * a group running two tasks has two.
 */

/** One task under the step or group. */
export type WorkflowInspectorTask = {
  id: string;
  title: string;
  /** What it is doing, in the draft's own word — `working`, `done`, `dropped`. */
  said: string;
  /** Short facts: `3 files`, `12 turns`, `$0.31`. Values, never sentences. */
  facts?: readonly string[];
  /** Why it is worth a second look — a task a later task edited after it finished. */
  flag?: string;
};

/** One Check at the boundary. */
export type WorkflowInspectorCheck = {
  name: string;
  /** What it came to. Absent where it has not run. */
  outcome?: string;
  named?: FactChipNamed;
};

/** One case that runs at this boundary. Drawn apart from the Checks above. */
export type WorkflowInspectorTest = {
  id: string;
  title: string;
  /** What the run came to, or why there is nothing to run. */
  outcome?: string;
  named?: FactChipNamed;
};

/** A Drone a redirect could reach. */
export type WorkflowInspectorDrone = { id: string; label: string };

export type WorkflowInspectorRedirect = Omit<DroneMessageBoxProps, "placeholder"> & {
  /**
   * The Drones this box could reach. **One is a sentence, several are a
   * picker** — a group running two tasks has two Drones, and a box that did
   * not say which one it reached would send a correction to either.
   */
  drones: readonly WorkflowInspectorDrone[];
  /** Which one it is addressed to. */
  reaches?: string;
  onReaches?: (id: string) => void;
};

export type WorkflowInspectorProps = {
  /** The step's label, or the group's name. */
  name: string;
  kind: "step" | "group";
  /** What it is doing now, as a sentence. */
  doing: string;
  tasks?: readonly WorkflowInspectorTask[];
  /** Why there are no tasks, where there are none. */
  tasksAbsent?: string;
  checks?: readonly WorkflowInspectorCheck[];
  checksAbsent?: string;
  tests?: readonly WorkflowInspectorTest[];
  testsAbsent?: string;
  redirect?: WorkflowInspectorRedirect;
  /** Hold to stop what is running here. Absent where nothing is running. */
  stop?: Pick<HoldButtonProps, "children" | "askLabel" | "description" | "onCommit" | "onAsk" | "disabled" | "pending">;
};

function Region({ name, children }: { name: string; children: React.ReactNode }) {
  return (
    <section className="armada-wf-inspector__region" aria-label={name}>
      <h4 className="armada-wf-inspector__eyebrow">{name}</h4>
      {children}
    </section>
  );
}

function Absent({ said }: { said: string }) {
  return (
    <p className="armada-wf-inspector__absent" role="note">
      {said}
    </p>
  );
}

export function WorkflowInspector({
  name,
  kind,
  doing,
  tasks = [],
  tasksAbsent,
  checks = [],
  checksAbsent,
  tests = [],
  testsAbsent,
  redirect,
  stop,
}: WorkflowInspectorProps) {
  return (
    <div className="armada-wf-inspector" aria-label={`${name}, ${kind}`} role="region">
      <header className="armada-wf-inspector__head">
        <h3 className="armada-wf-inspector__name">{name}</h3>
        <p className="armada-wf-inspector__doing">{doing}</p>
      </header>

      <Region name="Tasks">
        {tasks.length === 0 ? (
          <Absent said={tasksAbsent ?? "No tasks are recorded here."} />
        ) : (
          <ul className="armada-wf-inspector__tasks">
            {tasks.map((task) => (
              <li className="armada-wf-inspector__task" key={task.id}>
                <span className="armada-wf-inspector__task-name">{task.title}</span>
                <span className="armada-wf-inspector__values">
                  <FactChip>{task.said}</FactChip>
                  {(task.facts ?? []).map((fact) => (
                    <FactChip key={fact}>{fact}</FactChip>
                  ))}
                </span>
                {task.flag === undefined ? null : (
                  <p className="armada-wf-inspector__flag" role="note">
                    {task.flag}
                  </p>
                )}
              </li>
            ))}
          </ul>
        )}
      </Region>

      <Region name="Checks at this boundary">
        {checks.length === 0 ? (
          <Absent said={checksAbsent ?? "No Check runs here."} />
        ) : (
          <ul className="armada-wf-inspector__rows">
            {checks.map((check) => (
              <li className="armada-wf-inspector__row" key={check.name}>
                <span className="armada-wf-inspector__row-name">{check.name}</span>
                <FactChip named={check.named}>{check.outcome ?? "not run"}</FactChip>
              </li>
            ))}
          </ul>
        )}
      </Region>

      {/* Apart from the Checks above, and never folded into them. */}
      <Region name="Tests at this boundary">
        {tests.length === 0 ? (
          <Absent said={testsAbsent ?? "No case is owed here."} />
        ) : (
          <ul className="armada-wf-inspector__rows">
            {tests.map((test) => (
              <li className="armada-wf-inspector__row" key={test.id}>
                <span className="armada-wf-inspector__row-name">{test.title}</span>
                <FactChip named={test.named}>{test.outcome ?? "not covered"}</FactChip>
              </li>
            ))}
          </ul>
        )}
      </Region>

      {redirect === undefined ? null : (
        <Region name="Redirect">
          {redirect.drones.length === 0 ? null : redirect.drones.length === 1 ? (
            <p className="armada-wf-inspector__reaches" role="note">
              Reaches {redirect.drones[0]?.label}
            </p>
          ) : (
            <Select
              label="Reaches"
              value={redirect.reaches}
              onChange={(event) => redirect.onReaches?.(event.currentTarget.value)}
            >
              {redirect.drones.map((drone) => (
                <option value={drone.id} key={drone.id}>
                  {drone.label}
                </option>
              ))}
            </Select>
          )}
          <DroneMessageBox
            value={redirect.value}
            onChange={redirect.onChange}
            onSend={redirect.onSend}
            disabled={redirect.disabled}
            disabledReason={redirect.disabledReason}
            waiting={redirect.waiting}
          />
        </Region>
      )}

      {stop === undefined ? null : (
        <div className="armada-wf-inspector__acts">
          <HoldButton {...stop} />
        </div>
      )}
    </div>
  );
}
