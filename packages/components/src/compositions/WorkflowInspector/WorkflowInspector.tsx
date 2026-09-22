import { DroneMessageBox, type DroneMessageBoxProps } from "../DroneMessageBox/DroneMessageBox";
import { FactChip, type FactChipNamed } from "../FactChip/FactChip";
import { HoldButton, type HoldButtonProps } from "../../primitives/HoldButton/HoldButton";
import { PathChip } from "../PathChip/PathChip";
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

/** One line a task's own agent wrote, newest last. */
export type WorkflowInspectorLine = {
  id: string;
  /** When it was said, already formatted. */
  at?: string;
  said: string;
};

/**
 * What one task is, read whole. **Drawn only for `kind: "task"`** — a step and
 * a group carry tasks, and a task carries what it was told and what it wrote.
 */
export type WorkflowInspectorTaskReading = {
  /** What its Drone was told — the planner's words, never a paraphrase. */
  brief?: string;
  briefAbsent?: string;
  /** The repository-relative paths it claims. */
  scope?: readonly string[];
  scopeAbsent?: string;
  /** The tasks it runs beside, by id. Empty is a task that runs alone. */
  beside?: readonly string[];
  /** The last thing it wrote — a path, and what the edit was. */
  lastEdit?: { path: string; says?: string };
  lastEditAbsent?: string;
  /** Its own lines. Bounded by the caller, which says what it left out. */
  log?: readonly WorkflowInspectorLine[];
  logAbsent?: string;
};

export type WorkflowInspectorProps = WorkflowInspectorTaskReading & {
  /** The step's label, the group's name, or the task's id and title. */
  name: string;
  kind: "step" | "group" | "task";
  /** What it is doing now, as a sentence. */
  doing: string;
  tasks?: readonly WorkflowInspectorTask[];
  /** Why there are no tasks, where there are none. */
  tasksAbsent?: string;
  checks?: readonly WorkflowInspectorCheck[];
  checksAbsent?: string;
  tests?: readonly WorkflowInspectorTest[];
  testsAbsent?: string;
  /**
   * The boundary this reading sits behind, where it failed: the Check that
   * broke it, how many times it has been run again, and what the next Drone is
   * told. Absent everywhere else.
   */
  failure?: { says: string; retrySays?: string; toldNext?: string };
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

/** The directory half of a path, trailing separator kept — `PathChip`'s rule. */
function splitPath(path: string): { directory?: string; basename: string } {
  const cut = path.lastIndexOf("/");
  if (cut < 0) return { basename: path };
  return { directory: path.slice(0, cut + 1), basename: path.slice(cut + 1) };
}

/** What a task was told, what it claims, what it wrote and what it said. */
function TaskRegions({ reading }: { reading: WorkflowInspectorTaskReading }) {
  const beside = reading.beside ?? [];
  const scope = reading.scope ?? [];
  const log = reading.log ?? [];
  return (
    <>
      <Region name="What its Drone was told">
        {reading.brief === undefined ? (
          <Absent said={reading.briefAbsent ?? "No brief was recorded for this task."} />
        ) : (
          <p className="armada-wf-inspector__brief">{reading.brief}</p>
        )}
      </Region>

      <Region name="What it may touch">
        {scope.length === 0 ? (
          <Absent said={reading.scopeAbsent ?? "The planner named no files for this task."} />
        ) : (
          <ul className="armada-wf-inspector__scope">
            {scope.map((path) => (
              <li key={path}>
                <PathChip {...splitPath(path)} title={path} />
              </li>
            ))}
          </ul>
        )}
      </Region>

      <Region name="What it runs beside">
        {beside.length === 0 ? (
          <Absent said="Nothing else in its group runs at the same time." />
        ) : (
          <p className="armada-wf-inspector__beside">{beside.join(", ")}</p>
        )}
      </Region>

      <Region name="Its last edit">
        {reading.lastEdit === undefined ? (
          <Absent said={reading.lastEditAbsent ?? "Nothing this task wrote has been read yet."} />
        ) : (
          <PathChip
            {...splitPath(reading.lastEdit.path)}
            title={reading.lastEdit.path}
            {...(reading.lastEdit.says === undefined ? {} : { note: reading.lastEdit.says })}
          />
        )}
      </Region>

      <Region name="Its log">
        {log.length === 0 ? (
          <Absent said={reading.logAbsent ?? "This task has said nothing yet."} />
        ) : (
          <ul className="armada-wf-inspector__log">
            {log.map((line) => (
              <li key={line.id}>
                {line.at === undefined ? null : (
                  <span className="armada-wf-inspector__at mono">{line.at}</span>
                )}
                <span className="armada-wf-inspector__said">{line.said}</span>
              </li>
            ))}
          </ul>
        )}
      </Region>
    </>
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
  failure,
  redirect,
  stop,
  ...reading
}: WorkflowInspectorProps) {
  return (
    <div className="armada-wf-inspector" aria-label={`${name}, ${kind}`} role="region">
      <header className="armada-wf-inspector__head">
        <h3 className="armada-wf-inspector__name">{name}</h3>
        <p className="armada-wf-inspector__doing">{doing}</p>
      </header>

      {failure === undefined ? null : (
        <Region name="Why this boundary stopped">
          <p className="armada-wf-inspector__failed">{failure.says}</p>
          {failure.retrySays === undefined ? null : (
            <FactChip named="failed">{failure.retrySays}</FactChip>
          )}
          {failure.toldNext === undefined ? null : (
            <pre className="armada-wf-inspector__told">{failure.toldNext}</pre>
          )}
        </Region>
      )}

      {kind === "task" ? <TaskRegions reading={reading} /> : null}

      {kind === "task" ? null : (
      <>
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
      </>
      )}

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
