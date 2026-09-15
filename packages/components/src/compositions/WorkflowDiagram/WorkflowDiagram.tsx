import type { ReactNode } from "react";

/**
 * Workflow diagram — one box per step, left to right, and the gate between
 * them. Job detail draws this in place of the idle step view while a Job
 * waits for approval, since nothing has run yet to draw a result for.
 *
 * **One box per step, never the gate machine** — a box lists only the step's
 * own Checks and what it asks the Judge, never the tiers, rulings or retries
 * a rail draws once a Job is running.
 */

/** One Check a step declares. A command, and where it covers less than
 * everything, what it covers. */
export type WorkflowDiagramCheck = {
  command: string;
  covers?: string;
};

/** One thing the step asks of the Judge — counts, never the question. */
export type WorkflowDiagramDeclaration = {
  label: string;
};

/** Whether the gate after a step moves the Job on its own or waits for a
 * person. `auto` and `auto_if_judge_passes` both move on their own —
 * `gateLabel` is where the two are told apart, in words. */
export type WorkflowDiagramGate = "auto" | "person";

export type WorkflowDiagramLoop = {
  /** The id of the step this loop returns to. */
  to: string;
  /** The pass cap, in words — `up to 5 passes`. */
  label: string;
};

export type WorkflowDiagramStep = {
  id: string;
  /** The step's name, in sans. */
  label: ReactNode;
  /** Whether `label` is a `step_id` rather than a name, so it renders in mono. */
  labelIsAnIdentifier?: boolean;
  /** The Checks this step declares. Absent is ungated. */
  checks?: WorkflowDiagramCheck[];
  /** What this step asks the Judge. Absent is a step the Judge never looks at. */
  declarations?: WorkflowDiagramDeclaration[];
  /** How the Job leaves this step — the marker drawn after its box. */
  gate: WorkflowDiagramGate;
  /**
   * The gate's own word — `the checks decide, unless the Judge objects`,
   * `a person answers`. Absent on `auto`, which draws no row, on the rail's
   * own rule: a marker on every step would bury the two that matter.
   */
  gateLabel?: string;
  /** Where this step closes a loop, and its cap. Absent on every step that
   * sends nothing back, which is most steps of most workflows. */
  loop?: WorkflowDiagramLoop;
};

export type WorkflowDiagramProps = {
  steps: WorkflowDiagramStep[];
};

/** Two grid columns per step: the box, then the gate after it. The last
 * step's gate column carries no arrow, since nothing follows it. */
function columnsOf(count: number): string {
  return Array.from({ length: count }, () => "minmax(11rem, 1fr) auto").join(" ");
}

export function WorkflowDiagram({ steps }: WorkflowDiagramProps) {
  const loops = steps.flatMap((step, from) => {
    if (step.loop === undefined) return [];
    const to = steps.findIndex((candidate) => candidate.id === step.loop?.to);
    return to === -1 ? [] : [{ from, to, label: step.loop.label }];
  });

  return (
    <div className="armada-diagram" style={{ gridTemplateColumns: columnsOf(steps.length) }}>
      {steps.map((step, i) => (
        <div className="armada-diagram__step" style={{ gridColumn: i * 2 + 1, gridRow: 1 }} key={step.id}>
          <span className="armada-diagram__name" data-identifier={step.labelIsAnIdentifier || undefined}>
            {step.label}
          </span>
          {step.checks === undefined && step.declarations === undefined ? null : (
            <ul className="armada-diagram__declared">
              {(step.checks ?? []).map((check, c) => (
                <li className="armada-diagram__row" key={`check-${c}`}>
                  <span className="armada-diagram__command">{check.command}</span>
                  {check.covers === undefined ? null : (
                    <span className="armada-diagram__covers">{check.covers}</span>
                  )}
                </li>
              ))}
              {(step.declarations ?? []).map((declared, d) => (
                <li className="armada-diagram__row" key={`declared-${d}`}>
                  <span className="armada-diagram__command">{declared.label}</span>
                </li>
              ))}
            </ul>
          )}
        </div>
      ))}
      {steps.map((step, i) => {
        const last = i === steps.length - 1;
        if (last && step.gateLabel === undefined) return null;
        return (
          <div
            className="armada-diagram__gate"
            data-waits={step.gate === "person" || undefined}
            style={{ gridColumn: i * 2 + 2, gridRow: 1 }}
            key={`gate-${step.id}`}
          >
            {last ? null : (
              <span className="armada-diagram__gate-arrow" aria-hidden>
                →
              </span>
            )}
            {step.gateLabel === undefined ? null : (
              <span className="armada-diagram__gate-label">{step.gateLabel}</span>
            )}
          </div>
        );
      })}
      {loops.map((loop, l) => (
        <div
          className="armada-diagram__loop"
          style={{ gridColumn: `${loop.to * 2 + 1} / ${loop.from * 2 + 2}`, gridRow: 2 }}
          key={`loop-${l}`}
        >
          <div className="armada-diagram__loop-bracket" />
          <span className="armada-diagram__loop-label">{loop.label}</span>
        </div>
      ))}
    </div>
  );
}
