import type { ReactNode } from "react";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { TaskMark, type TaskMarkState } from "../TaskMark/TaskMark";

/**
 * One plan task's whole reading, on the layer that can hold it.
 *
 * **A new home rather than an edit**, `JobHoldsSheet`'s shape. The task's note,
 * the paths it touches and both ends of its evidence were drawn under the title
 * in the rail, and the rail is 380px: one real plan put 5,196 characters of
 * them under 364 characters of title, and a 69-character path was clipped
 * mid-word because a path cannot break. The row keeps what a person scans —
 * mark, id, title, and how many files — and this holds the rest.
 *
 * **Not `wide`.** `JobHoldsSheet` argues it for the same reading: a card
 * legible in a 380px column, spread across three times that, puts four figures
 * across a field instead of down one.
 */
export type PlanTaskSheetProps = {
  open: boolean;
  /** `T1`, `T2`, … as the plan numbered it. */
  id: string;
  title: string;
  state: TaskMarkState;
  /** Present on a dropped task and on nothing else. */
  reason?: string;
  /** What the other fields cannot hold. Absent where the task has none. */
  note?: string;
  /** The repository-relative paths the task names, in the order it named them. */
  scope?: readonly string[];
  /**
   * Which of `scope` the work actually reached, and what it reached that
   * `scope` never named. **Absent is not empty**: absent is a Job whose turns
   * were not read, where empty is a task that touched nothing. `#1432`.
   */
  touched?: { declared: readonly { path: string; touched: boolean }[]; unplanned: readonly string[] };
  /** What the plan said should prove it, written before the work. */
  expects?: string;
  /** What the work said proved it, written by whoever did it. */
  shown?: string;
  /** The window is at `--window-floor`. */
  floor?: boolean;
  onClose?: () => void;
};

/**
 * The count over the file list. **Three figures only where the work has been
 * read**, since a Job whose turns nobody read would otherwise say every
 * declared file went untouched.
 */
function filesSaid(
  declared: number,
  touched?: PlanTaskSheetProps["touched"],
): string {
  if (touched === undefined) return `Files · ${declared}`;
  const reached = touched.declared.filter((file) => file.touched).length;
  const unplanned = touched.unplanned.length;
  const said = [`declared ${declared}`, `touched ${reached}`];
  if (unplanned > 0) said.push(`unplanned ${unplanned}`);
  return `Files · ${said.join(" · ")}`;
}

/** The words for a state, as the rail's own mark spells them. */
const STATE_SAID: Record<TaskMarkState, string> = {
  open: "Open",
  working: "Working",
  done: "Done",
  dropped: "Dropped",
};

export function PlanTaskSheet({
  open,
  id,
  title,
  state,
  reason,
  note,
  scope = [],
  touched,
  expects,
  shown,
  floor = false,
  onClose,
}: PlanTaskSheetProps) {
  const nothing =
    (note ?? "") === "" && scope.length === 0 && (expects ?? "") === "" && (shown ?? "") === "";
  return (
    <Sheet
      open={open}
      contained
      floor={floor}
      title={title}
      subtitle={
        <span className="armada-task-sheet__said">
          <span className="armada-task-sheet__id">{id}</span>
          <span className="armada-task-sheet__state">{STATE_SAID[state]}</span>
        </span>
      }
      controls={<TaskMark state={state} />}
      closeLabel="Close"
      closeBinding="Esc"
      onClose={onClose}
    >
      <div className="armada-task-sheet__body">
        {reason === undefined ? null : (
          <Field label="Dropped because">
            <p className="armada-task-sheet__prose">{reason}</p>
          </Field>
        )}
        {(note ?? "") === "" ? null : (
          <Field label="Note">
            <p className="armada-task-sheet__prose">{note}</p>
          </Field>
        )}
        {scope.length === 0 && (touched?.unplanned.length ?? 0) === 0 ? null : (
          <Field label={filesSaid(scope.length, touched)}>
            <ul className="armada-task-sheet__files">
              {(touched?.declared ?? scope.map((path) => ({ path, touched: true }))).map((file) => (
                <li key={file.path}>
                  <span>{file.path}</span>
                  {/* **Only the unreached one is marked.** Marking both halves
                      would put a badge on every row and say nothing; the row
                      worth stopping on is the one the work never reached. */}
                  {file.touched ? null : (
                    <span className="armada-task-sheet__unreached">not touched</span>
                  )}
                </li>
              ))}
              {(touched?.unplanned ?? []).map((path) => (
                <li key={path} className="armada-task-sheet__unplanned">
                  <span>{path}</span>
                  <span className="armada-task-sheet__unreached">not planned</span>
                </li>
              ))}
            </ul>
          </Field>
        )}
        {(expects ?? "") === "" && (shown ?? "") === "" ? null : (
          <Field label="Evidence">
            {/* **Two rows and never one.** The plan names an artifact before the
                work starts and the work finds out what actually proved it; the
                pair disagreeing is what a reader is here for. */}
            <dl className="armada-task-sheet__evidence">
              <Evidence said="Expects" of={expects} absent="Nothing was named." />
              <Evidence
                said="Shown"
                of={shown}
                absent={state === "done" ? "Nothing was recorded." : "Not yet."}
              />
            </dl>
          </Field>
        )}
        {nothing ? (
          <p className="armada-task-sheet__absent" role="note">
            This task says nothing beyond its title.
          </p>
        ) : null}
      </div>
    </Sheet>
  );
}

function Field({ label, children }: { label: string; children: ReactNode }) {
  return (
    <section className="armada-task-sheet__field">
      <h3 className="armada-task-sheet__label">{label}</h3>
      {children}
    </section>
  );
}

function Evidence({ said, of, absent }: { said: string; of?: string; absent: string }) {
  const empty = (of ?? "") === "";
  return (
    <div className="armada-task-sheet__row">
      <dt>{said}</dt>
      <dd className={empty ? "armada-task-sheet__unsaid" : undefined}>{empty ? absent : of}</dd>
    </div>
  );
}
