import { Input } from "../../primitives/Input/Input";

/**
 * One thing the Job is held to, with where its words came from.
 *
 * **Where they came from is not how they are answered.** A criterion read out
 * of an issue may be answered by a Check, and one you typed may be answered by
 * the Judge; the two facts are drawn as two, because collapsing them is what
 * would make "from the issue" sound like a verdict.
 */
export type ProposalCriterion = {
  /** Stable for the life of the proposal, where anything has minted one. */
  id?: string;
  text: string;
  /** Where the words came from, as a person reads it. */
  origin: string;
  /** How it is answered — a Check, the Judge, or somebody attesting. */
  verifiedBy: string;
  /**
   * The issue these words came from has been edited since the Job froze them.
   *
   * **The Job keeps what it froze** (#1530, 22 Sep). This says the source has
   * moved; it never replaces the words, and nothing here re-reads the issue.
   */
  movedSince?: string;
};

export type ProposalDoneWhenProps = {
  criteria: readonly ProposalCriterion[];
  /** One criterion reworded. Absent draws them frozen, which is after approval. */
  onCriterion?: (at: number, text: string) => void;
  /** Said under the list. What the Judge marks against, and when. */
  said: string;
};

export function ProposalDoneWhen({ criteria, onCriterion, said }: ProposalDoneWhenProps) {
  return (
    <section className="armada-proposal__region" aria-label="Done when">
      <h3 className="armada-proposal__heading">Done when</h3>
      {criteria.length === 0 ? (
        <p className="armada-proposal__said">
          Nothing was read out of a request or an issue, so this Job is held to the workflow
          alone.
        </p>
      ) : (
        <ul className="armada-proposal__criteria">
          {criteria.map((criterion, at) => (
            <li className="armada-proposal__criterion" key={criterion.id ?? at}>
              {onCriterion === undefined ? (
                <p className="armada-proposal__criterion-text">{criterion.text}</p>
              ) : (
                <Input
                  label={`Criterion ${at + 1}`}
                  value={criterion.text}
                  onChange={(event) => onCriterion(at, event.target.value)}
                />
              )}
              <p className="armada-proposal__criterion-origin">
                {`${criterion.origin} · answered by the ${criterion.verifiedBy}`}
              </p>
              {criterion.movedSince === undefined ? null : (
                <p className="armada-proposal__moved" role="note">
                  {`The issue has been edited since these words were frozen — last on ${criterion.movedSince}. The Job is held to the words above.`}
                </p>
              )}
            </li>
          ))}
        </ul>
      )}
      <p className="armada-proposal__said">{said}</p>
    </section>
  );
}
