import type { ReactNode } from "react";

/**
 * Console output — a Check's own stdout, read where the Check is.
 *
 * **A Check's output is an artifact and this is its renderer.** The screen
 * showed that Checks passed or failed and gave no way to see what they wrote,
 * which is unauditable the moment a suite goes green for the wrong reason. The
 * run log already exists on disk: Bridge reads and tails it, keeps nothing,
 * and it is cleaned up when the Job is. No new store, no retention policy.
 *
 * **Verbatim output is never glossed.** A line is quoted exactly as it was
 * written — which is why the identifier a suite prints appears bare here and
 * nowhere else on the screen. Anything the surface has to add is a `note`
 * beside the line or a `marker` row of its own, never words inserted into
 * one.
 *
 * **A fold is a run of lines standing as one.** 2,180 lines is not a reading;
 * the compile is 62 of them and nobody opened the Job for it. A fold carries
 * its own line count so what is hidden is stated rather than implied — and a
 * search that could not reach inside one would be the failure every code host
 * ships.
 *
 * **The comparison is emitted, not reconstructed.** A line present at the
 * parent commit and absent here is drawn as a deletion because the check ran
 * at both commits and said so; Bridge is not diffing two stored logs. That is
 * the whole value of exposing output, because the transcript alone still says
 * `315 passed`.
 *
 * **Following is a state of this component and pauses on a scroll.** A stream
 * that scrolls itself cannot be read — the same rule `ActivityLogSheet` holds
 * one layer out, spelled the same way.
 */

/** One row of the reading: a line of the file, or a fold standing for many. */
export type ConsoleRow = ConsoleLine | ConsoleFold;

export type ConsoleLine = {
  row: "line";
  /**
   * The line's number in the whole file, never in the shown region. A region
   * is a window onto a file and a citation names the file's own numbering.
   */
  at: number;
  /** The line, verbatim. Mono, and never rewritten. */
  text?: ReactNode;
  /**
   * How this line stands against the run at the parent commit.
   *
   * `absent` is a line that ran there and does not exist here — drawn as a
   * deletion, in the diff tokens, because that is exactly what it is.
   * `marker` is the surface's own row saying where the difference falls; it
   * has no counterpart in the file and carries no quoted text.
   */
  against?: "absent" | "marker";
  /**
   * What the surface adds beside the line — `ran and passed at HEAD~1 · never
   * ran here`. Set back, and never inside the quoted text.
   */
  note?: ReactNode;
  /** The line a selector landed on, so a link restores the reading it named. */
  landed?: boolean;
};

export type ConsoleFold = {
  row: "fold";
  /** The number of the first line the fold stands for. */
  at: number;
  /**
   * What is inside it, and how much — `compiling chrono-lite v0.4.2 (18.4s) —
   * 62 lines`. **The count is not optional prose.** A fold that did not say
   * what it swallowed is a reading with a hole in it.
   */
  says: ReactNode;
  /** Open folds render their lines as ordinary rows beneath them. */
  open?: boolean;
  onToggle?: () => void;
};

/**
 * The tools, present only where the viewer is at a size that can hold them.
 *
 * **They are not on the inline size on purpose.** Inline composes the page and
 * a tool bar there would make a quoted region look like a place to work.
 */
export type ConsoleTools = {
  /**
   * What is being searched for and what it found. **`found` says how many are
   * inside folds**, because a search that reported 3 and showed 1 is the
   * failure this component exists not to have.
   */
  find?: { term: ReactNode; found?: ReactNode };
  /** Cycling the differences — `1 of 1 difference`. Absent where there are none. */
  differences?: { says: ReactNode; onPrevious?: () => void; onNext?: () => void };
  /** Wrap, Raw, Reveal in Finder. Controls the caller owns and this does not name. */
  acts?: ReactNode;
};

/**
 * Where the shown region sits in the file it came from.
 *
 * **A truncated reading must say it is truncated.** `lines 1,940–1,962 of
 * 2,180` above a transcript is what stops a person concluding the file is
 * short, and the path is what they take to a shell when it is not enough.
 */
export type ConsoleRegion = {
  /** `lines 1,940–1,962 of 2,180`, or `1,124 of 1,204 · 6.1s elapsed` while tailing. */
  says: ReactNode;
  /** Where the output was written, relative to the Job's own directory. Mono. */
  path?: string;
  /** What it weighs — `4.1MB`. Absent while the check is still writing it. */
  size?: ReactNode;
  /** Why the reading is where it is — `jumped to first difference`. */
  why?: ReactNode;
};

export type ConsoleOutputProps = {
  rows: ConsoleRow[];
  region?: ConsoleRegion;
  tools?: ConsoleTools;
  /**
   * True while the check is still writing. The last line moves, there is no
   * verdict yet, and the region line sits under the reading rather than over
   * it — beside the moving edge, which is the part being watched.
   */
  following?: boolean;
  /** What a reading with nothing in it says. Never an empty frame. */
  emptyNote?: ReactNode;
};

export function ConsoleOutput({
  rows,
  region,
  tools,
  following = false,
  emptyNote,
}: ConsoleOutputProps) {
  const bar = region === undefined ? null : <Region region={region} />;
  return (
    <div className="armada-console" data-following={following ? "true" : undefined}>
      {tools === undefined ? null : <Tools tools={tools} />}
      {following ? null : bar}
      {rows.length === 0 ? (
        <p className="armada-console__empty">{emptyNote}</p>
      ) : (
        <ol className="armada-console__rows">
          {rows.map((row) =>
            row.row === "fold" ? (
              <li className="armada-console__fold" key={`fold-${row.at}`}>
                <span className="armada-console__at">{row.at}</span>
                {/* The chevron is text rather than a glyph: it sits in the
                    mono column at the transcript's own size, and the registry
                    reserves chevron-down to the surfaces that carry one. */}
                <button
                  type="button"
                  className="armada-console__fold-says"
                  aria-expanded={row.open === true}
                  onClick={row.onToggle}
                >
                  <span className="armada-console__caret" aria-hidden>
                    {row.open === true ? "▾" : "▸"}
                  </span>
                  {row.says}
                </button>
              </li>
            ) : (
              <li
                className="armada-console__line"
                key={`line-${row.at}`}
                data-against={row.against}
                data-landed={row.landed ? "true" : undefined}
              >
                <span className="armada-console__at">{row.at}</span>
                <span className="armada-console__text">
                  {row.text}
                  {row.note === undefined ? null : (
                    <span className="armada-console__note">{row.note}</span>
                  )}
                </span>
              </li>
            ),
          )}
        </ol>
      )}
      {following ? bar : null}
    </div>
  );
}

function Region({ region }: { region: ConsoleRegion }) {
  return (
    <div className="armada-console__region">
      <span className="armada-console__where">{region.says}</span>
      {region.path === undefined ? null : (
        // The whole path stays in the title however narrow the viewer gets,
        // the way the rail's own output path does: a path clipped from the
        // right with nothing behind it is a path that is gone.
        <span className="armada-console__path" title={region.path}>
          {region.path}
        </span>
      )}
      {region.size === undefined ? null : (
        <span className="armada-console__size">{region.size}</span>
      )}
      {region.why === undefined ? null : <span className="armada-console__why">{region.why}</span>}
    </div>
  );
}

function Tools({ tools }: { tools: ConsoleTools }) {
  return (
    <div className="armada-console__tools">
      {tools.find === undefined ? null : (
        <span className="armada-console__find">
          <span className="armada-console__term">{tools.find.term}</span>
          {tools.find.found === undefined ? null : (
            <span className="armada-console__found">{tools.find.found}</span>
          )}
        </span>
      )}
      {tools.differences === undefined ? null : (
        <span className="armada-console__differences">
          <span className="armada-console__difference-says">{tools.differences.says}</span>
          <button
            type="button"
            className="armada-console__step"
            onClick={tools.differences.onPrevious}
          >
            Previous
          </button>
          <button type="button" className="armada-console__step" onClick={tools.differences.onNext}>
            Next
          </button>
        </span>
      )}
      {tools.acts === undefined ? null : (
        <span className="armada-console__acts">{tools.acts}</span>
      )}
    </div>
  );
}
