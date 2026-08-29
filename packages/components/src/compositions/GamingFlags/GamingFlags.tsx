import type { ReactNode } from "react";

import { Prose } from "../../primitives/Prose/Prose";

/**
 * One pattern the gaming check flagged on a step's evidence, and where.
 *
 * **A flag is not a verdict and never borrows one's treatment.** `circle-*` is
 * reserved to the Judge's criterion verdicts and `shield-*` to the Checks; a
 * flag is neither, so these rows are label-only for the reason a declaration
 * row is, and `flag` itself is spent on the stopped step's own mark on the
 * rail.
 *
 * **Nor is it a failure.** `evidence_suspect` routes as its own escalation
 * rather than a gate failure, precisely because resubmission under the same
 * instructions would likely reproduce the gaming — so these rows carry weight
 * rather than `--step-failed`, and a reader is not told a criterion was
 * refused when what happened is that the evidence is not trusted.
 */
export type GamingFlag = {
  /**
   * The pattern, spelled as the workflow's `flag_if` spells it —
   * `check_config_edited`, `assertion_weakened`. Mono, because no vocabulary
   * in the repository carries a verb per gaming pattern and the wire spelling
   * is what renders. Reported.
   */
  pattern: string;
  /**
   * The file, line or assertion the flag is about. **The whole value of the
   * finding** — an uncited flag is unactionable exactly as an uncited refusal
   * is. Absent where Fleet flagged and cited nothing, which draws no slot
   * rather than an empty one.
   */
  cited?: string;
};

/**
 * How much of the citation this surface is for reading.
 *
 * **Two, and the difference is the surface's width rather than a preference.**
 * `clipped` is the rail's row: one line, the rest in the title, because a rail
 * is a column of pointers. `whole` is the override dialog's: the citation is
 * the thing a person is taking responsibility for, and one they cannot read is
 * a decision taken blind — so it wraps, and it goes through `Prose`, because a
 * citation is written by a model and arrives with paths and expressions in it.
 */
export type GamingFlagCitation = "clipped" | "whole";

export type GamingFlagsProps = {
  /** In the order the check answered. Empty draws nothing. */
  flags: GamingFlag[];
  /**
   * What the rows are, said once over them rather than once on each — a step
   * can trip several patterns and the source is the same machine every time.
   * Omitted where the surface already named it in the sentence above.
   */
  said?: ReactNode;
  /** Defaults to the rail's row, which is where most of them are read. */
  citation?: GamingFlagCitation;
};

/**
 * The flags on one step, as rows.
 *
 * **One component and not two renders of one shape.** Two flags used to reach
 * the override dialog as a single sentence assembled from their fields —
 * "It flagged X in Y, Z in W." — while the rail drew the same two as rows. The
 * wire has had `pattern` and `cited` as separate fields the whole time, so the
 * sentence was structure being thrown away and then read back out of prose.
 * Drawing it once is what stops the two surfaces from disagreeing about what a
 * flag looks like.
 */
export function GamingFlags({ flags, said, citation = "clipped" }: GamingFlagsProps) {
  if (flags.length === 0) return null;
  return (
    <div className="armada-gaming-flags" data-citation={citation}>
      {said === undefined ? null : <span className="armada-gaming-flags__said">{said}</span>}
      <ul className="armada-gaming-flags__list">
        {flags.map((flag, at) => (
          <li className="armada-gaming-flags__flag" key={`flag-${at}`}>
            <span className="armada-gaming-flags__pattern">{flag.pattern}</span>
            {flag.cited === undefined || flag.cited === "" ? null : citation === "whole" ? (
              <div className="armada-gaming-flags__cited">
                <Prose text={flag.cited} />
              </div>
            ) : (
              // The whole citation stays in the title however narrow the row
              // gets, the way the Check's output path does.
              <span className="armada-gaming-flags__cited" title={flag.cited}>
                {flag.cited}
              </span>
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}
