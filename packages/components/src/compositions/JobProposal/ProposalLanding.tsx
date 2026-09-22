import { Input } from "../../primitives/Input/Input";
import { Select } from "../../primitives/Select/Select";

/**
 * How the work reaches the repository.
 *
 * **Four controls, and no shape** (#1530, 22 Sep). Where it lands, what a
 * branch is cut per, what counts as finished, and whether the pull request is
 * offered or parked. Nothing here names Atomic, Complex or Convoy: a screen
 * says what a Job does, never what kind it is.
 *
 * **From and Lands in are both here and they are not the same field.** They
 * differ when you start from an unmerged branch or land in a long-lived one.
 */
export type ProposalLandingValue = {
  /** Where the work lands. Empty is the Manifest naming no base. */
  target: string;
  /** Where it starts. Empty reads as the same as `target`. */
  from: string;
  /** One branch per Job, or one per group. Per-task branches are dropped. */
  branching: "job" | "group";
  /** What has to happen before the Job counts as finished. */
  completeWhen: string;
  /** Whether the pull request is offered for review or parked as a draft. */
  prMode: "ready" | "draft";
};

/** One answer to "complete when", with whether Fleet can tell yet. */
export type CompleteChoice = {
  value: string;
  label: string;
  /**
   * Whether anything on the record answers it today. **Drawn, not hidden** —
   * an option a person may pick and Fleet cannot observe is a fact about this
   * milestone rather than a control to leave out.
   */
  served: boolean;
};

export type ProposalLandingProps = {
  landing: ProposalLandingValue;
  /** Absent draws every value frozen, which is what approval does to them. */
  onLanding?: (landing: ProposalLandingValue) => void;
  completeChoices: readonly CompleteChoice[];
};

/** What each branching unit is called where it is read rather than chosen. */
const BRANCHING: Record<ProposalLandingValue["branching"], string> = {
  job: "One branch for the whole Job",
  group: "One branch per group",
};

const PR_MODE: Record<ProposalLandingValue["prMode"], string> = {
  ready: "Offered for review",
  draft: "Parked as a draft",
};

export function ProposalLanding({ landing, onLanding, completeChoices }: ProposalLandingProps) {
  const moved = (change: Partial<ProposalLandingValue>): void =>
    onLanding?.({ ...landing, ...change });
  const chosen = completeChoices.find((one) => one.value === landing.completeWhen);
  return (
    <section className="armada-proposal__region" aria-label="How it lands">
      <h3 className="armada-proposal__heading">How it lands</h3>
      {onLanding === undefined ? (
        <dl className="armada-proposal__frozen-fields">
          <Frozen label="From" value={landing.from === "" ? "The Manifest names no base" : landing.from} />
          <Frozen
            label="Lands in"
            value={landing.target === "" ? "The Manifest names no base" : landing.target}
          />
          <Frozen label="Branches" value={BRANCHING[landing.branching]} />
          <Frozen label="Complete when" value={chosen?.label ?? landing.completeWhen} />
          <Frozen label="Pull request" value={PR_MODE[landing.prMode]} />
        </dl>
      ) : (
        <div className="armada-proposal__fields">
          <Input label="From" value={landing.from} mono onChange={(event) => moved({ from: event.target.value })} />
          <Input
            label="Lands in"
            value={landing.target}
            mono
            onChange={(event) => moved({ target: event.target.value })}
          />
          <Select
            label="Branches"
            value={landing.branching}
            onChange={(event) =>
              moved({ branching: event.target.value as ProposalLandingValue["branching"] })
            }
          >
            <option value="job">{BRANCHING.job}</option>
            <option value="group">{BRANCHING.group}</option>
          </Select>
          <Select
            label="Complete when"
            value={landing.completeWhen}
            onChange={(event) => moved({ completeWhen: event.target.value })}
          >
            {completeChoices.map((choice) => (
              <option key={choice.value} value={choice.value}>
                {choice.label}
              </option>
            ))}
          </Select>
          <Select
            label="Pull request"
            value={landing.prMode}
            onChange={(event) =>
              moved({ prMode: event.target.value as ProposalLandingValue["prMode"] })
            }
          >
            <option value="ready">{PR_MODE.ready}</option>
            <option value="draft">{PR_MODE.draft}</option>
          </Select>
        </div>
      )}
      {/* An answer nothing on the record can observe yet says so where it is
          chosen, rather than looking like a setting that does something. */}
      {chosen === undefined || chosen.served ? null : (
        <p className="armada-proposal__said" role="note">
          {`Nothing on a Job's record answers "${chosen.label}" yet, so Armada cannot tell you when it has happened.`}
        </p>
      )}
    </section>
  );
}

function Frozen({ label, value }: { label: string; value: string }) {
  return (
    <div className="armada-proposal__frozen-field">
      <dt>{label}</dt>
      <dd>{value}</dd>
    </div>
  );
}
