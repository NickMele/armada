import { Checkbox } from "../../primitives/Checkbox/Checkbox";
import { Button } from "../../primitives/Button/Button";

/**
 * One step's gate: **three independent boxes, and a fourth state the boxes
 * cannot express** (#1548).
 *
 * Checks, Judge and You are three facts about one step and not three points on
 * a scale, so they are boxes rather than a menu — "a Judge reads this, and no
 * Check runs on it" is a sentence the menu could not say. The fourth state is
 * the repository deciding, which belongs to no box because it is not this
 * Job's answer at all; overriding it hands the step back to the three.
 */
export type ProposalGateRow = {
  /** The step's own id, as the workflow declares it. */
  id: string;
  /** What the step is called. */
  label: string;
  checks: boolean;
  judge: boolean;
  you: boolean;
  /**
   * The repository policy that decides this step, where one does and this Job
   * has not taken it over. The policy's own name — `auto_merge`, `review_gate`.
   */
  repositoryDecides?: string;
  /** Whether this Job took the decision off the repository for itself. */
  overridden?: boolean;
  /** The `advance_gate` this combination is on the wire today. */
  advanceGate: string;
  /** What Fleet does with it, in one sentence. */
  does: string;
  /** Fleet has nothing to do with this combination yet. Said, never hidden. */
  unmeant?: boolean;
};

/** Which of the three boxes moved. */
export type GateBox = "checks" | "judge" | "you";

export type ProposalGatesProps = {
  steps: readonly ProposalGateRow[];
  /** One box moved on one step. Absent draws every gate as a frozen reading. */
  onGate?: (stepId: string, box: GateBox, ticked: boolean) => void;
  /** Take the decision off the repository for this Job, or hand it back. */
  onOverride?: (stepId: string, overridden: boolean) => void;
  /** The line the ticks cannot turn off, said once above the steps. */
  alwaysLooks: string;
};

export function ProposalGates({ steps, onGate, onOverride, alwaysLooks }: ProposalGatesProps) {
  return (
    <section className="armada-proposal__region" aria-label="What each step is gated by">
      <h3 className="armada-proposal__heading">What each step is gated by</h3>
      {/* Above the steps rather than on each of them: it is the same sentence
          about every one, and repeated per row it would read as a property of
          the row that some other row might not have. */}
      <p className="armada-proposal__always" role="note">
        {alwaysLooks}
      </p>
      <ul className="armada-proposal__gates">
        {steps.map((step) => (
          <li className="armada-proposal__gate" key={step.id} aria-label={step.label}>
            <div className="armada-proposal__gate-head">
              <span className="armada-proposal__gate-step">{step.label}</span>
              <span className="armada-proposal__wire" title="advance_gate">
                {step.advanceGate}
              </span>
            </div>
            {step.repositoryDecides !== undefined && step.overridden !== true ? (
              <Deferred
                step={step}
                {...(onOverride === undefined ? {} : { onOverride })}
              />
            ) : (
              <Boxes step={step} {...(onGate === undefined ? {} : { onGate })} />
            )}
            <p className="armada-proposal__gate-does">{step.does}</p>
            {step.unmeant !== true ? null : (
              <p className="armada-proposal__unmeant" role="note">
                Fleet does nothing with this combination yet.
              </p>
            )}
            {step.repositoryDecides === undefined || step.overridden !== true ? null : (
              <p className="armada-proposal__overrode">
                {`This Job decides this step for itself, in place of the repository's ${step.repositoryDecides}.`}
                {onOverride === undefined ? null : (
                  <Button
                    variant="secondary"
                    size="sm"
                    onClick={() => onOverride(step.id, false)}
                  >
                    Give it back to the repository
                  </Button>
                )}
              </p>
            )}
          </li>
        ))}
      </ul>
    </section>
  );
}

/** The three boxes, ticked or read. */
function Boxes({
  step,
  onGate,
}: {
  step: ProposalGateRow;
  onGate?: (stepId: string, box: GateBox, ticked: boolean) => void;
}) {
  const boxes: [GateBox, string, boolean][] = [
    ["checks", "Checks", step.checks],
    ["judge", "Judge", step.judge],
    ["you", "You", step.you],
  ];
  // Frozen, the ticks are a reading rather than a control: a disabled checkbox
  // is a control somebody will press, and what it says about a Job that has
  // already started is a fact.
  if (onGate === undefined) {
    const on = boxes.filter(([, , ticked]) => ticked).map(([, label]) => label);
    return (
      <p className="armada-proposal__gate-frozen">
        {on.length === 0 ? "Nobody looks" : on.join(" · ")}
      </p>
    );
  }
  return (
    <div className="armada-proposal__boxes">
      {boxes.map(([box, label, ticked]) => (
        <Checkbox
          key={box}
          checked={ticked}
          aria-label={`${label} on ${step.label}`}
          onChange={(event) => onGate(step.id, box, event.target.checked)}
        >
          {label}
        </Checkbox>
      ))}
    </div>
  );
}

/** The fourth state: the repository decides, and this Job may take it over. */
function Deferred({
  step,
  onOverride,
}: {
  step: ProposalGateRow;
  onOverride?: (stepId: string, overridden: boolean) => void;
}) {
  return (
    <div className="armada-proposal__deferred">
      <span className="armada-proposal__deferred-said">
        {`The repository decides — ${step.repositoryDecides}`}
      </span>
      {onOverride === undefined ? null : (
        <Button variant="secondary" size="sm" onClick={() => onOverride(step.id, true)}>
          Decide it for this Job
        </Button>
      )}
    </div>
  );
}
