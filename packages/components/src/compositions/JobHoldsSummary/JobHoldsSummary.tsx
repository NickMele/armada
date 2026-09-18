import { FigureList, type Figure } from "../FigureList/FigureList";

/**
 * The Job's pulse: the last thing anyone did on it, and what it holds on this
 * machine, with the full reading one press away.
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
 * **The top is the last thing anyone did, from any voice.** Fleet, the Drone
 * and a person all act on a Job, and what a reader wants from this block is
 * the latest of them, not the last lines of one. The whole log is a press away.
 *
 * **Every line of it is one list.** The top row is a `FigureList` row like the
 * three under it, so the labels start on one edge and the values end on one;
 * its own markup is what the right edge left behind, and the owner saw it.
 *
 * **Absence still speaks, in less room.** A read that has not answered and a
 * Job that holds nothing are different things — `JobResources` is careful about
 * that and so is this. `figures` is `null` for the first and `note` says which;
 * the second is a reading, with `None` against the processes and the worktree
 * saying what became of it.
 *
 * **The worktree is one row and not two, since #1484.** It drew `on disk` over
 * `Size on disk 1.2 GiB` — and `on disk` was a constant, because nothing looks
 * at a worktree unless somebody presses `Look now` in the sheet. The size is
 * the row now, and what a look finds wrong replaces it.
 *
 * **No `Look now` here, and no button.** Going and looking is an act on the
 * reading, and the reading is in the sheet. The control that opens it sits on
 * the region's title line, where the run keeps its elapsed figure.
 *
 * **What the Job is spending reads here too, since #1484.** Spend and Turns
 * were the second line of the job header, above everything a person opened the
 * Job for; this is the region that already answers what the Job is taking, so
 * they read beside the processes and the worktree. They come from the Job
 * rather than from a look, so they are drawn above the machine figures and
 * outside what `Read … ago` qualifies.
 */

/** One line of what happened on this machine, as the caller formatted it. */
export type HoldsLine = {
  /** The wall clock — `14:31:58`. */
  at: string;
  /** Who wrote it — `Fleet`, `Drone`. */
  actor: string;
  /** What it said, in a few words. Held to two lines rather than growing. */
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
  /**
   * The worktree in one value: what it takes on disk with its unit resolved —
   * `1.2 GiB`, never a byte count — or what is wrong with it, `gone`,
   * `could not be read`, `none on disk`.
   *
   * **A size and a fault are the same slot because they are the same
   * question.** What a person wants off this row is whether the checkout is
   * there and what it is costing; a figure answers both, and a fault replaces
   * it rather than standing beside it — `gone · 1.2 GiB` would be a size for a
   * directory that is not there.
   */
  worktree: string;
  /** Whether that value is a fault. */
  worktreeIsWrong?: boolean;
};

export type JobHoldsSummaryProps = {
  /** The last thing anyone did on this Job. Absent where nothing has happened yet. */
  latest?: HoldsLine;
  /** Why there is nothing to show, where there is nothing. */
  latestNote?: string;
  /**
   * The reading's figures, or `null` where none has arrived.
   *
   * **`null` is not empty**, and `note` is what says which.
   */
  figures: HoldsFigures | null;
  /** Why there is no reading, where there is none. */
  note?: string;
  /** How old the reading is, as a phrase, like `4s`. Formatted by the caller. */
  age?: string;
  /**
   * What the Job has cost, hedged by the caller — `at least ~$1.96`. Absent on
   * a Fleet that does not price, which draws no row rather than a zero.
   */
  spend?: string;
  /** Turns taken against turns allowed — `82 of 300`. Absent draws no row. */
  turns?: string;
};

export function JobHoldsSummary({ latest, latestNote, figures, note, age, spend, turns }: JobHoldsSummaryProps) {
  /**
   * Every row of the block, in one list, so the latest event lands on the same
   * two edges as the figures under it — the label's on the left and the
   * value's on the right. It was this block's own markup until the figures went
   * to the right edge and left it behind; a second alignment for the line at
   * the top is the drift `FigureList` was made to stop.
   */
  const rows: Figure[] = [
    ...(latest === undefined
      ? []
      : [{ label: latest.actor, value: latest.said, detail: latest.at, words: true, wrong: latest.wrong }]),
    ...(spend === undefined ? [] : [{ label: "Spend", value: spend }]),
    ...(turns === undefined ? [] : [{ label: "Turns", value: turns }]),
    ...(figures === null
      ? []
      : [
          {
            label: "Processes",
            value: figures.processes === 0 ? "None" : String(figures.processes),
            wrong: figures.processes === 0 && figures.nothingRunningIsWrong,
          },
          { label: "Worktree", value: figures.worktree, wrong: figures.worktreeIsWrong },
        ]),
  ];
  return (
    <section className="armada-holds-summary">
      {latest === undefined ? <p className="armada-holds-summary__note">{latestNote ?? NOTHING_RECORDED}</p> : null}
      {rows.length === 0 ? null : <FigureList figures={rows} />}
      {figures === null ? <p className="armada-holds-summary__note">{note ?? NOTHING_READ_YET}</p> : null}
      {/* The instant qualifies the machine figures, which is why it is here
          and not a caption: a process can exit between the reading and this
          screen. Spend and Turns above them are the Job's own and are not what
          this dates. Absent rather than guessed where the caller has no age. */}
      {age === undefined || figures === null ? null : (
        <div className="armada-holds-summary__foot">
          <span className="armada-holds-summary__read-at">{`Read ${age} ago`}</span>
        </div>
      )}
    </section>
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
const NOTHING_READ_YET = "Not read yet";
