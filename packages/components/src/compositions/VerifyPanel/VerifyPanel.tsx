import type { ReactNode } from "react";
import { Button } from "../../primitives/Button/Button";
import { FactChip } from "../FactChip/FactChip";

/**
 * Verify — Journey 9's live dry-run: setup and every Check in this checkout,
 * once each, one after another.
 *
 * **An act behind its own button, and never automatic.** Opening the surface
 * does not press it and neither does drift, because it runs a real test suite.
 *
 * **A rehearsal, so nothing is hued.** A step's exit code is a `FactChip`
 * beside the code it expects, as a single run's is: no step is kept as a pass
 * or a fail, and nothing reaches Doctor.
 *
 * **It reports and never fixes**, and it says what it says nothing about.
 * **Data in, callbacks out** — no protocol type.
 */

export type VerifyPanelStepState =
  | { kind: "waiting" }
  | { kind: "running"; elapsed: ReactNode }
  | { kind: "ran"; result: ReactNode; duration: ReactNode }
  | { kind: "not_run"; why: ReactNode };

export type VerifyPanelStep = {
  id: string;
  /** The group it is drawn under — `Setup` or `Checks`. */
  group: string;
  name: ReactNode;
  /** Its `run` line, exactly as declared. Mono. */
  run: string;
  state: VerifyPanelStepState;
};

export type VerifyPanelProps = {
  /** The latest Verify's steps, in order. Absent: none, or it was put away. */
  steps?: VerifyPanelStep[];
  /** One unhued line of counts, once it has ended. */
  ended?: ReactNode;
  /** Press Verify. **Absent where it cannot start now.** */
  onVerify?: () => void;
  /** Why Verify cannot start now, where that is not simply that one is out. */
  unavailable?: ReactNode;
  /** Why the last press was refused, in Fleet's words. */
  refused?: ReactNode;
  /** Stop the step that is out, which ends the Verify. */
  onStop?: () => void;
  /** Put an ended Verify away. Its steps stay under Earlier runs. */
  onDismiss?: () => void;
};

export function VerifyPanel({
  steps = [],
  ended,
  onVerify,
  unavailable,
  refused,
  onStop,
  onDismiss,
}: VerifyPanelProps) {
  const groups = [...new Set(steps.map((step) => step.group))].map(
    (group) => [group, steps.filter((step) => step.group === group)] as const,
  );

  return (
    <section className="armada-verify-panel" aria-label="Verify">
      <div className="armada-verify-panel__head">
        <span className="armada-verify-panel__title">Verify</span>
        <p className="armada-verify-panel__says">
          Runs setup and every Check once, one after another, in this checkout as it is on disk.
        </p>
        <span className="armada-verify-panel__acts">
          {onStop === undefined ? null : (
            <Button variant="secondary" size="sm" onClick={onStop}>
              Stop
            </Button>
          )}
          {ended === undefined || onDismiss === undefined ? null : (
            <Button variant="ghost" size="sm" onClick={onDismiss}>
              Dismiss
            </Button>
          )}
          <Button variant="secondary" size="sm" disabled={onVerify === undefined} onClick={onVerify}>
            Verify
          </Button>
        </span>
      </div>

      {onVerify !== undefined || unavailable === undefined ? null : (
        <p className="armada-verify-panel__note">{unavailable}</p>
      )}
      {refused === undefined ? null : <p className="armada-verify-panel__note">{refused}</p>}

      {groups.length === 0 ? null : (
        <div className="armada-verify-panel__groups">
          {groups.map(([group, members]) => (
            <div className="armada-verify-panel__group" key={group}>
              <span className="armada-verify-panel__group-label">{group}</span>
              <ol className="armada-verify-panel__steps">
                {members.map((step) => (
                  <li className="armada-verify-panel__step" key={step.id}>
                    <span className="armada-verify-panel__name">{step.name}</span>
                    <span className="armada-verify-panel__state">{stateOf(step.state)}</span>
                    <code className="armada-verify-panel__run">{step.run}</code>
                  </li>
                ))}
              </ol>
            </div>
          ))}
        </div>
      )}

      {ended === undefined ? null : <p className="armada-verify-panel__ended">{ended}</p>}

      <p className="armada-verify-panel__scope">
        Each step is a run like any other, with its log and what it changed under Earlier runs, and
        none is kept as a pass or a fail. Verify says nothing about policy or permissions, which
        name nothing runnable.
      </p>
    </section>
  );
}

function stateOf(state: VerifyPanelStepState): ReactNode {
  switch (state.kind) {
    case "waiting":
      return "not reached yet";
    case "running":
      return <>running now · {state.elapsed}</>;
    case "ran":
      return (
        <>
          <FactChip>{state.result}</FactChip> {state.duration}
        </>
      );
    case "not_run":
      return <>did not run — {state.why}</>;
  }
}
