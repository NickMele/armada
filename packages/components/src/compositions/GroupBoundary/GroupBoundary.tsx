import { FactChip, type FactChipNamed } from "../FactChip/FactChip";
import { StepBar, type TaskBarSegment } from "../StepBar/StepBar";

/**
 * A group's boundary — what runs once every task in the group has stopped, and
 * what it came to. `#1536`.
 *
 * **The Checks run at each group's end** (`#1530`, 21 Sep), so the bar belongs
 * to the group and not to the step: one segment per Check, and the four
 * readings a person waits on are not run, one in flight, all passed, and one
 * failed with the group stopped behind it.
 *
 * **The tests are a region apart** (`#1530`, 22 Sep), carrying their own reason
 * for being empty — an empty list there would read as "none owed".
 *
 * **It composes no sentence.** Every line of prose is the caller's.
 */

/** What one Check at the boundary came to. Spelled as the wire spells it. */
export type GroupBoundaryCheckReads = "not run" | "running" | "passed" | "failed";

export type GroupBoundaryCheck = {
  /** The Check's declared name — `screens_test`, `typecheck`. */
  name: string;
  reads: GroupBoundaryCheckReads;
};

/** One case that runs at this same boundary, drawn apart from the Checks. */
export type GroupBoundaryTest = {
  id: string;
  /** The spec's repository path, or the case's own id where it has none. */
  spec: string;
  /** What the run came to, or why there is nothing to run — `not covered`. */
  reads: string;
  named?: FactChipNamed;
};

export type GroupBoundaryProps = {
  /** `7 checks ran at this boundary`, in the tense the group's state earns. */
  says: string;
  checks: readonly GroupBoundaryCheck[];
  /** Why no Check runs here. Drawn instead of the bar, never beside an empty one. */
  checksAbsent?: string;
  /** What the boundary came to — `all seven passed`, `screens_test failed`. */
  verdictSays?: string;
  verdictNamed?: FactChipNamed;
  /** `second run`, where the group has been run again. Absent on the first. */
  retrySays?: string;
  /** The commit the group left. Absent until it left one. */
  commit?: string;
  /**
   * What stopping means here — `no task of group 4 starts until this passes`.
   * One group at a time is the rule this line is the only evidence of.
   */
  stopsSays?: string;
  /**
   * What the next Drone is told, verbatim: the failed Check's own output. A
   * retry given a paraphrase is a retry working from something nobody can check.
   */
  toldNext?: string;
  /** `1 test ran at this boundary`. Absent where none does. */
  testsSay?: string;
  tests?: readonly GroupBoundaryTest[];
  /** Why there is no case here. **Required**, for the reason above. */
  testsAbsent: string;
};

/** A Check's reading, on the bar's own segment grammar. */
const SEGMENT: Record<GroupBoundaryCheckReads, TaskBarSegment> = {
  "not run": "open",
  running: "working",
  passed: "done",
  failed: "failed",
};

/** The hue a Check's chip takes. Only the two that are a verdict take one. */
const NAMED: Partial<Record<GroupBoundaryCheckReads, FactChipNamed>> = {
  passed: "passed",
  failed: "failed",
};

export function GroupBoundary({
  says,
  checks,
  checksAbsent,
  verdictSays,
  verdictNamed,
  retrySays,
  commit,
  stopsSays,
  toldNext,
  testsSay,
  tests = [],
  testsAbsent,
}: GroupBoundaryProps) {
  return (
    <div className="armada-boundary">
      <section className="armada-boundary__region" aria-label="Checks at this boundary">
        <p className="armada-boundary__head">
          <span className="armada-boundary__says">{says}</span>
          {verdictSays === undefined ? null : <FactChip named={verdictNamed}>{verdictSays}</FactChip>}
          {retrySays === undefined ? null : <FactChip>{retrySays}</FactChip>}
          {commit === undefined ? null : <FactChip title={commit}>{commit}</FactChip>}
        </p>
        {checks.length === 0 ? (
          <p className="armada-boundary__absent" role="note">
            {checksAbsent ?? "No Check runs at this group's end."}
          </p>
        ) : (
          <>
            <StepBar tasks={checks.map((check) => SEGMENT[check.reads])} label={says} />
            <ul className="armada-boundary__checks">
              {checks.map((check) => (
                <li key={check.name} data-reads={check.reads}>
                  <span className="armada-boundary__check mono">{check.name}</span>
                  <FactChip named={NAMED[check.reads]}>{check.reads}</FactChip>
                </li>
              ))}
            </ul>
          </>
        )}
        {stopsSays === undefined ? null : (
          <p className="armada-boundary__stops" role="note">
            {stopsSays}
          </p>
        )}
        {toldNext === undefined ? null : (
          <div className="armada-boundary__told">
            <span className="armada-boundary__eyebrow">What the next Drone is told</span>
            <pre className="armada-boundary__output">{toldNext}</pre>
          </div>
        )}
      </section>

      {/* Apart from the Checks above, and never folded into them. */}
      <section className="armada-boundary__region" aria-label="Tests at this boundary">
        {tests.length === 0 ? (
          <p className="armada-boundary__absent" role="note">
            {testsAbsent}
          </p>
        ) : (
          <>
            {testsSay === undefined ? null : <p className="armada-boundary__says">{testsSay}</p>}
            <ul className="armada-boundary__tests">
              {tests.map((test) => (
                <li key={test.id}>
                  <span className="armada-boundary__check mono">{test.spec}</span>
                  <FactChip named={test.named}>{test.reads}</FactChip>
                </li>
              ))}
            </ul>
          </>
        )}
      </section>
    </div>
  );
}
