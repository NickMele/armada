import { Button } from "../../primitives/Button/Button";

/**
 * What this Job holds on the machine, in a few lines, with the full reading one
 * press away.
 *
 * **The run is what a person opens a Job to read.** `JobResources` is a card
 * with a verdict, a look, a process table and a disk figure, and above the run
 * it answered a question nobody had asked yet. So the reading moved to a sheet
 * and this took its place: five lines that say whether anything is wrong, and a
 * control for when the answer is yes.
 *
 * **It reads after the run, not before it.** It is context for the steps rather
 * than a preface to them.
 *
 * **One region for *what is happening on this machine right now*, not two.**
 * The Fleet log had its own region above the run drawing the same answer, so
 * its tail is the first thing here and the standalone region is gone.
 *
 * **Absence still speaks, in less room.** A read that has not answered and a
 * Job that holds nothing are different things — `JobResources` is careful about
 * that and so is this. `figures` is `null` for the first and `note` says which;
 * the second is a reading, with `None` against the processes and the worktree
 * saying what became of it.
 *
 * **No `Look now` here.** Going and looking is an act on the reading, and the
 * reading is in the sheet. This block's only control is the one that opens it.
 */

/** One line of what happened on this machine, as the caller formatted it. */
export type HoldsLine = {
  /** The wall clock — `14:31:58`. */
  at: string;
  /** Who wrote it — `Fleet`, `Drone`. */
  actor: string;
  /** What it said, in a few words. One line, clipped rather than wrapped. */
  said: string;
  /** Whether the line records something going wrong. */
  wrong?: boolean;
};

/** The figures, where a reading has arrived. */
export type HoldsFigures = {
  /** How many processes this Job holds. `0` is an answer, and reads `None`. */
  processes: number;
  /**
   * Whether holding none is a fault here. **The caller decides**, from the
   * examination the full reading decides it from — a second judgment made in
   * this block would disagree with the sheet it opens.
   */
  nothingRunningIsWrong?: boolean;
  /** The worktree in a phrase — `healthy`, `gone`, `could not be read`. */
  worktree: string;
  /** Whether that phrase is a fault. */
  worktreeIsWrong?: boolean;
  /**
   * What the worktree takes on disk, unit resolved — never a byte count.
   * Absent where there is nothing on disk to size, and then the row is absent
   * too: the worktree line above already says what became of it, and a size of
   * nothing beside it would be a blank value drawn twice.
   */
  size?: string;
};

export type JobHoldsSummaryProps = {
  /**
   * The last lines the machine wrote, newest first. **Two are drawn**, because
   * the region this replaced grew to 15rem and pushed the run off the screen.
   */
  tail: HoldsLine[];
  /** Why there are no lines, where there are none. */
  tailNote?: string;
  /**
   * The reading's figures, or `null` where none has arrived.
   *
   * **`null` is not empty**, and `note` is what says which.
   */
  figures: HoldsFigures | null;
  /** Why there is no reading, where there is none. */
  note?: string;
  /** How old the reading is, as a phrase — `4s`. Formatted by the caller. */
  age?: string;
  /** Opens the full reading. The only act this block carries. */
  onOpen: () => void;
};

export function JobHoldsSummary({
  tail,
  tailNote,
  figures,
  note,
  age,
  onOpen,
}: JobHoldsSummaryProps) {
  const lines = tail.slice(0, 2);
  return (
    <section className="armada-holds-summary">
      {lines.length === 0 ? (
        <p className="armada-holds-summary__note">{tailNote ?? NOTHING_RECORDED}</p>
      ) : (
        <ul className="armada-holds-summary__tail">
          {lines.map((line, at) => (
            <li className="armada-holds-summary__line" key={at} data-wrong={line.wrong || undefined}>
              <span className="armada-holds-summary__at">{line.at}</span>
              <span className="armada-holds-summary__actor">{line.actor}</span>
              <span className="armada-holds-summary__said">{line.said}</span>
            </li>
          ))}
        </ul>
      )}

      {figures === null ? (
        <p className="armada-holds-summary__note">{note ?? NOTHING_READ_YET}</p>
      ) : (
        <dl className="armada-holds-summary__figures">
          <Figure
            label="Processes"
            value={figures.processes === 0 ? "None" : String(figures.processes)}
            wrong={figures.processes === 0 && figures.nothingRunningIsWrong}
          />
          <Figure label="Worktree" value={figures.worktree} wrong={figures.worktreeIsWrong} />
          {figures.size === undefined ? null : (
            <Figure label="Size on disk" value={figures.size} />
          )}
        </dl>
      )}

      <div className="armada-holds-summary__foot">
        {/* The instant qualifies every figure above it, which is why it is here
            and not a caption — a process can exit between the reading and this
            screen. Absent rather than guessed where the caller has no age. */}
        {age === undefined || figures === null ? null : (
          <span className="armada-holds-summary__read-at">{`Read ${age} ago`}</span>
        )}
        <Button size="sm" ground="sunken" onClick={onOpen}>
          Open the full reading
        </Button>
      </div>
    </section>
  );
}

/** A label and its figure, on one row. The figure is mono and right-aligned. */
function Figure({ label, value, wrong }: { label: string; value: string; wrong?: boolean }) {
  return (
    <div className="armada-holds-summary__figure" data-wrong={wrong || undefined}>
      <dt className="armada-holds-summary__label">{label}</dt>
      <dd className="armada-holds-summary__value">{value}</dd>
    </div>
  );
}

/**
 * What stands in for the tail when nothing has been recorded.
 *
 * **A Job Armada has done nothing to yet is a real answer**, so it is a
 * sentence rather than two empty rows — an empty stream under a heading is how
 * a socket that never opened went unnoticed.
 */
const NOTHING_RECORDED = "Armada has not recorded anything about this Job yet.";

/** What stands in for the figures when no reading has arrived. */
const NOTHING_READ_YET = "Nothing has been read from this machine yet.";
