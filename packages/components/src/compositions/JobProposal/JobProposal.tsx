import { Input } from "../../primitives/Input/Input";
import { DroneCap } from "../DispatchSettings/DroneCap";
import { TierModels, TIERS } from "../DispatchSettings/TierModels";
import type { TierChoice } from "../DispatchSettings/TierModels";
import { ProposalDoneWhen } from "./ProposalDoneWhen";
import type { ProposalCriterion } from "./ProposalDoneWhen";
import { ProposalGates } from "./ProposalGates";
import type { GateBox, ProposalGateRow } from "./ProposalGates";
import { ProposalLanding } from "./ProposalLanding";
import type { CompleteChoice, ProposalLandingValue } from "./ProposalLanding";

export type { ProposalCriterion } from "./ProposalDoneWhen";
export type { GateBox, ProposalGateRow } from "./ProposalGates";
export type { CompleteChoice, ProposalLandingValue } from "./ProposalLanding";

/**
 * A Job at its approval gate, and the same Job one press later.
 *
 * **Two of the three moments the classifying screen has** (#1541). The first —
 * Armada still reading the request — happens before a Job exists at all and is
 * drawn on the dispatch form, where the request being read still is. What is
 * here is the answer: everything the proposer chose, with nothing frozen, and
 * then the same values with the instant they froze at.
 *
 * **The difference between the two is which callbacks arrive.** A handler
 * absent is what freezes its control, rather than a `readOnly` flag beside
 * each one — a screen that can hand in an `onGate` and a `frozenAt` together
 * is a screen that can draw a frozen gate somebody can still move.
 */
export type JobProposalProps = {
  /** The title the proposer answered, and yours until you approve. */
  title: string;
  onTitle?: (title: string) => void;
  /** The workflow it chose, by the name its own file declares. */
  workflow: string;
  /** What the workflow's steps are gated by, in the order they run. */
  steps: readonly ProposalGateRow[];
  onGate?: (stepId: string, box: GateBox, ticked: boolean) => void;
  onOverride?: (stepId: string, overridden: boolean) => void;
  /** The line no tick turns off. */
  alwaysLooks: string;
  tiers: TierChoice;
  onTiers?: (tiers: TierChoice) => void;
  models: readonly string[];
  /** How many Drones this Job may run at once. Absent is the machine's cap holding. */
  droneCap?: number;
  onDroneCap?: (cap: number | undefined) => void;
  /** How many the machine runs across every Job. `null` before Fleet said. */
  machineCap: number | null;
  landing: ProposalLandingValue;
  onLanding?: (landing: ProposalLandingValue) => void;
  completeChoices: readonly CompleteChoice[];
  criteria: readonly ProposalCriterion[];
  onCriterion?: (at: number, text: string) => void;
  /**
   * When this was approved, written out. **Absent is a proposal nobody has
   * approved**, which is the whole of the difference between the two moments.
   */
  frozenAt?: string;
};

/** What the screen says it is, at each of the two moments. */
const SAID = {
  open: "Nothing here is decided yet. Change any of it, and approving is what freezes it.",
  frozen: "Frozen when you approved it. The Job runs on these values and nothing re-reads them.",
};

export function JobProposal({
  title,
  onTitle,
  workflow,
  steps,
  onGate,
  onOverride,
  alwaysLooks,
  tiers,
  onTiers,
  models,
  droneCap,
  onDroneCap,
  machineCap,
  landing,
  onLanding,
  completeChoices,
  criteria,
  onCriterion,
  frozenAt,
}: JobProposalProps) {
  const frozen = frozenAt !== undefined;
  return (
    <div className="armada-proposal">
      <header className="armada-proposal__head">
        {onTitle === undefined ? (
          <h2 className="armada-proposal__title">{title}</h2>
        ) : (
          <Input label="Title" value={title} onChange={(event) => onTitle(event.target.value)} />
        )}
        <p className="armada-proposal__workflow">
          {`${workflow} — ${steps.length} steps`}
        </p>
        <p className="armada-proposal__said">
          {frozen ? `${SAID.frozen} Approved ${frozenAt}.` : SAID.open}
        </p>
      </header>

      <ProposalGates
        steps={steps}
        {...(onGate === undefined ? {} : { onGate })}
        {...(onOverride === undefined ? {} : { onOverride })}
        alwaysLooks={alwaysLooks}
      />

      <section className="armada-proposal__region" aria-label="What each task runs on">
        <h3 className="armada-proposal__heading">What each task runs on</h3>
        {onTiers === undefined || onDroneCap === undefined ? (
          <>
            {/* The three tiers are read against each other, so they are one
                list; the cap is a different question and is its own. */}
            <dl className="armada-proposal__frozen-fields">
              {TIERS.map(([tier, label]) => (
                <div className="armada-proposal__frozen-field" key={tier}>
                  <dt>{label}</dt>
                  <dd>{tiers[tier] ?? "Auto — Armada picks it"}</dd>
                </div>
              ))}
            </dl>
            <dl className="armada-proposal__frozen-fields">
              <div className="armada-proposal__frozen-field">
                <dt>Drones at once</dt>
                <dd>
                  {droneCap === undefined
                    ? "As many as the machine allows"
                    : `${droneCap} of this machine's ${machineCap ?? "?"}`}
                </dd>
              </div>
            </dl>
          </>
        ) : (
          <>
            <TierModels
              tiers={tiers}
              onTiers={onTiers}
              models={models}
              said="The planner marks each task Difficult, Medium or Easy, and the model follows from this map. A tier left on Auto is Armada's to pick."
            />
            <DroneCap
              {...(droneCap === undefined ? {} : { cap: droneCap })}
              onCap={onDroneCap}
              machineCap={machineCap}
            />
          </>
        )}
      </section>

      <ProposalLanding
        landing={landing}
        {...(onLanding === undefined ? {} : { onLanding })}
        completeChoices={completeChoices}
      />

      <ProposalDoneWhen
        criteria={criteria}
        {...(onCriterion === undefined ? {} : { onCriterion })}
        said={
          frozen
            ? "The Judge marks against these words, as they were when you approved."
            : "Read out of the issue this work is linked to where there is one. The Judge marks against whatever these say when you approve."
        }
      />
    </div>
  );
}
