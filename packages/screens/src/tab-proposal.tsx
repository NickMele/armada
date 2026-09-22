// The moment between typing a prompt and a Job starting: what Armada made of
// the request, with nothing frozen until you approve it.
//
// **Two of the three states are here** (#1541). Armada still reading the
// request is drawn on the dispatch form, where the request is; this is the
// answer, and then the same values with the instant they froze at.
//
// **It takes Overview's place rather than a destination of its own.** A Job at
// its approval gate has entered no step and recorded nothing, so the
// arrangement Overview draws work in has no work to draw.
//
// **Nothing here reaches the wire.** `approve_dispatch` takes no body and no
// field carries a per-Job gate, so a person's changes live in this screen
// until the press. The shapes are the ones `#1545` promotes.

import { JobProposal } from "@armada/components";
import type { GateBox, ProposalLandingValue } from "@armada/components";
import type { JobDetail as JobWhole, JobSummary } from "@armada/protocol";

import { TAB_LABEL } from "./detail-tabs";
import { FLEET_ALWAYS_LOOKS } from "./draft/proposal";
import type { TierModels } from "./draft/proposal";
import {
  completeChoices,
  criteriaRowsOf,
  criteriaWith,
  frozenAtOf,
  gateRowsOf,
  gatesWith,
  landingValueOf,
  landingWith,
} from "./tab-proposal-read";
import type { ProposalEdits } from "./tab-proposal-read";

export type ProposalTabProps = {
  job: JobSummary;
  whole: JobWhole | null;
  /** What this Job is at, and what a person has moved since it arrived. */
  edits: ProposalEdits;
  /** One change, held by the screen — the whole of what an edit does today. */
  onEdits: (edits: ProposalEdits) => void;
  /** The models a tier may name. Empty until the connection answers. */
  models: readonly string[];
  /** Nothing may be moved while what is shown is not live. */
  stale: boolean;
};

export function ProposalTab({ job, whole, edits, onEdits, models, stale }: ProposalTabProps) {
  const { proposal, landing, criteria } = edits;
  const frozenAt = frozenAtOf(proposal);
  // Frozen is what a handler's absence means, and `stale` freezes the same
  // way: a control that sends nothing while the connection is down would take
  // an answer this window could not keep.
  const open = frozenAt === undefined && !stale;
  const moved = (change: Partial<ProposalEdits>): void => onEdits({ ...edits, ...change });

  return (
    <div className="armada-detail-tab" role="tabpanel" aria-label={TAB_LABEL.overview}>
      <JobProposal
        title={proposal.title}
        {...(open
          ? { onTitle: (title: string) => moved({ proposal: { ...proposal, title } }) }
          : {})}
        // The workflow's own name where Fleet holds it, and the id where it
        // does not — a Job naming a workflow this Fleet has no record of.
        workflow={job.workflow_id}
        steps={gateRowsOf(proposal.gates, whole)}
        {...(open
          ? {
              onGate: (stepId: string, box: GateBox, ticked: boolean) =>
                moved({
                  proposal: {
                    ...proposal,
                    gates: gatesWith(proposal.gates, stepId, { [box]: ticked }),
                  },
                }),
              // Taking the decision off the repository leaves the three boxes
              // as they were: the override says who decides, and answering it
              // for somebody would be this screen choosing.
              onOverride: (stepId: string, overridden: boolean) =>
                moved({
                  proposal: {
                    ...proposal,
                    gates: gatesWith(proposal.gates, stepId, { overridden }),
                  },
                }),
            }
          : {})}
        alwaysLooks={FLEET_ALWAYS_LOOKS}
        tiers={proposal.tiers}
        {...(open
          ? {
              onTiers: (tiers: TierModels) => moved({ proposal: { ...proposal, tiers } }),
              onDroneCap: (cap: number | undefined) =>
                moved({
                  proposal:
                    cap === undefined ? withoutCap(proposal) : { ...proposal, drone_cap: cap },
                }),
            }
          : {})}
        models={models}
        {...(proposal.drone_cap === undefined ? {} : { droneCap: proposal.drone_cap })}
        machineCap={proposal.machine_cap}
        landing={landingValueOf(landing)}
        {...(open
          ? {
              onLanding: (value: ProposalLandingValue) =>
                moved({ landing: landingWith(landing, value) }),
            }
          : {})}
        completeChoices={completeChoices()}
        criteria={criteriaRowsOf(criteria)}
        {...(open
          ? { onCriterion: (at: number, text: string) => moved({ criteria: criteriaWith(criteria, at, text) }) }
          : {})}
        {...(frozenAt === undefined ? {} : { frozenAt })}
      />
    </div>
  );
}

/**
 * The cap taken off again. **Absent, never zero** — a `drone_cap` of nothing
 * is the machine's own cap holding, and `0` would read as a Job allowed no
 * Drone at all.
 */
function withoutCap(proposal: ProposalEdits["proposal"]): ProposalEdits["proposal"] {
  const { drone_cap: _dropped, ...rest } = proposal;
  return rest;
}
