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
   * `check_config_edited`, `assertion_weakened`.
   *
   * **The fallback, not the reading.** `gaming_pattern` is a registered
   * vocabulary now and `verb` is what a person reads; this renders in mono
   * where the caller has no verb for the spelling that arrived, which is a
   * pattern the registry does not yet hold.
   */
  pattern: string;
  /**
   * What the pattern means, in words — `an assertion now asserts less`.
   *
   * **The caller supplies it, because this package cannot reach the registry.**
   * The verbs are generated into the app from `enum-verbs.toml`, and a copy
   * spelled here would be the second vocabulary that file exists to prevent.
   *
   * Present renders as prose and absent renders `pattern` in mono, unchanged.
   * That is not a style toggle: a verb is a sentence and a wire spelling is a
   * value, and rendering one as the other is how `assertion_weakened` came to
   * be printed at a person in the first place.
   */
  verb?: string;
  /**
   * The file, line or assertion the flag is about. **The whole value of the
   * finding** — an uncited flag is unactionable exactly as an uncited refusal
   * is. Absent where Fleet flagged and cited nothing, which draws no slot
   * rather than an empty one.
   */
  cited?: string;
  /**
   * Where the citation is, where Fleet could establish that from the patch.
   *
   * **Absent is a real answer and never a loading state.** Three flags can
   * never carry one: a finding about an absence has nowhere to point, a
   * citation about what a change does *not* do has nothing in the patch to
   * match, and a flag recorded before the field existed was never located. So
   * absent draws no link at all rather than a disabled one.
   *
   * **A line without a file is not a shape this can hold**, which is why it is
   * one object rather than two optional fields. `file` alone is ordinary: a
   * citation quoting a line the patch removed has no post-image number to
   * point at, and the file is still worth opening.
   */
  at?: GamingFlagAt;
  /**
   * The narrow question the check was actually asked about this pattern,
   * word for word — *Does this change alter an existing assertion so
   * that it asserts less than it did, and is that assertion made
   * nowhere else in this change?*
   *
   * The one thing on the row a flag can be judged against: a spelling
   * says what was looked for and a citation says what was seen, and
   * neither says what was claimed — so `assertion_weakened` over a
   * rustdoc sentence read as a finding until somebody spent an
   * afternoon on the diff proving it was not. The clauses are the whole
   * of it: the second is why a sentence restated elsewhere is not a
   * weakened assertion.
   */

  /**
   * Absent is an answer about the check rather than a gap: three
   * patterns are decided by reading the diff and nothing was asked, and
   * a flag recorded before Fleet kept this has none.
   *
   * Drawn only where the citation is read whole: a question is forty
   * words and the rail's row is one line of pointers, and the two
   * surfaces a flag is judged on both read whole — clipping a question
   * to eight words would put half a test on screen, the failure the row
   * already had.
   */
  asked?: string;
  /**
   * Where the whole exchange behind this flag is, relative to the repository
   * root. **The path, never the document**, exactly as a criterion's brief is.
   *
   * Absent wherever `asked` is, plus the flag Fleet could not write one for.
   * It draws as a control where `onOpenBrief` was given and as a value where
   * it was not, for `at`'s reason.
   */
  brief?: string;
};

/** Where in the change a flag points. */
export type GamingFlagAt = {
  /** Repository-relative, as the patch's post-image side spells it. */
  file: string;
  /** The line as this change leaves it. Absent where there is no post-image line. */
  line?: number;
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
  /**
   * Open the diff at a flag's location. Absent draws the location as a value
   * rather than a control — a surface that cannot open a diff should say where
   * the flag is anyway, and offering a press that goes nowhere is worse than
   * offering none.
   */
  onOpenAt?: (at: GamingFlagAt) => void;
  /**
   * Open the brief a flag was answered in, given its path. Absent draws the
   * brief as a value, for `onOpenAt`'s reason — and the value is still worth
   * drawing, because a path a person can find by hand beats knowing there was
   * an exchange and not where.
   */
  onOpenBrief?: (brief: string) => void;
};

/**
 * The flags on one step, as rows. One component and not two renders of
 * one shape: two flags used to reach the override dialog as a single
 * sentence assembled from their fields — "It flagged X in Y, Z in W."
 * — while the rail drew the same two as rows. The wire has had
 * `pattern` and `cited` as separate fields the whole time, so the
 * sentence was structure thrown away and read back out of prose.
 * Drawing it once stops the two surfaces disagreeing about what a flag
 * looks like.
 * A row says what was asked as well as what was found: six of the nine
 * patterns cost a model call against a question written to be argued
 * with, and the row drawing only the spelling and citation spent that
 * answer and discarded it — leaving a reader to re-derive it off the
 * diff, or take the flag on trust. #580.
 */
export function GamingFlags({
  flags,
  said,
  citation = "clipped",
  onOpenAt,
  onOpenBrief,
}: GamingFlagsProps) {
  if (flags.length === 0) return null;
  return (
    <div className="armada-gaming-flags" data-citation={citation}>
      {said === undefined ? null : <span className="armada-gaming-flags__said">{said}</span>}
      <ul className="armada-gaming-flags__list">
        {flags.map((flag, at) => (
          <li className="armada-gaming-flags__flag" key={`flag-${at}`}>
            {/* The verb where there is one, the wire spelling where there is
                not — and the two are not the same kind of thing, so they do
                not read the same. */}
            <span
              className="armada-gaming-flags__pattern"
              data-verb={flag.verb === undefined ? undefined : "true"}
            >
              {flag.verb ?? flag.pattern}
            </span>
            {/* Above the citation, because it is the question the citation is
                the answer to. Not through `Prose`: this is one sentence the
                registry holds, written by hand, and rendering it as markup
                would invite a check to word its own question in markup. */}
            {flag.asked === undefined || citation !== "whole" ? null : (
              <p className="armada-gaming-flags__asked">{flag.asked}</p>
            )}
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
            {flag.at == null ? null : <Where at={flag.at} onOpen={onOpenAt} />}
            {flag.brief === undefined ? null : (
              <Brief path={flag.brief} onOpen={onOpenBrief} />
            )}
          </li>
        ))}
      </ul>
    </div>
  );
}

/**
 * Where the flag is — `src/report.ts:41`, or the file alone.
 *
 * **Mono, because it is a path**, and it is the one thing on the row that goes
 * somewhere. It says so the way every openable path on this screen says so: no
 * glyph, the affordance token on hover, and nothing at all where the surface
 * gave it nowhere to go.
 */
function Where({ at, onOpen }: { at: GamingFlagAt; onOpen?: (at: GamingFlagAt) => void }) {
  // A file with no line is ordinary, not a partial answer: a citation quoting
  // a line the patch removed has no post-image number to point at.
  const where = at.line === undefined ? at.file : `${at.file}:${at.line}`;
  if (onOpen === undefined) {
    return (
      <span className="armada-gaming-flags__at" title={where}>
        {where}
      </span>
    );
  }
  return (
    <button
      type="button"
      className="armada-gaming-flags__at"
      data-opens="true"
      title={where}
      onClick={() => onOpen(at)}
    >
      {where}
    </button>
  );
}

/**
 * The brief the flag was answered in — `the brief implement.1.gaming.txt`.
 *
 * **The basename, with the whole path in the title.** Every one of these
 * begins `.armada/briefs/`, so a row set to the full path reads as a column of
 * the same eleven characters; the last segment names the step, the attempt and
 * the pattern, which is the half somebody is hunting for.
 *
 * **Named in words, because it sits beside a location that is also a path.**
 * Two mono paths under one finding say nothing about which is the change and
 * which is the exchange, and the one thing a reader is deciding here is which
 * of the two to open.
 */
function Brief({ path, onOpen }: { path: string; onOpen?: (brief: string) => void }) {
  const named = (
    <>
      <span className="armada-gaming-flags__brief-of">the brief</span>{" "}
      {path.slice(path.lastIndexOf("/") + 1)}
    </>
  );
  if (onOpen === undefined) {
    return (
      <span className="armada-gaming-flags__brief" title={path}>
        {named}
      </span>
    );
  }
  return (
    <button
      type="button"
      className="armada-gaming-flags__brief"
      data-opens="true"
      title={path}
      onClick={() => onOpen(path)}
    >
      {named}
    </button>
  );
}
