import { useState, type ReactNode } from "react";

import { ChevronDown, ChevronRight } from "lucide-react";

import { StepActivityMark, type StepActivity } from "../StepActivityMark/StepActivityMark";

/**
 * A step as one timeline: the phases in the order they happened, repeated for
 * each attempt the step was worked.
 *
 * **It replaces a graph and a list that said the same thing twice.** The strip
 * drew where the step stood and the story below it drew what happened there, so
 * a failed Check was a red node in one place and a printed exit code in
 * another. Here the phase *is* the row, and what happened in it opens beneath.
 *
 * **An attempt is a section, and the earlier ones fold.** A step handed back
 * twice has three sections; the one being worked is open and the rest carry
 * their outcome on a header a press opens. The strip could only ever say
 * "handed back twice" on a single edge, which is the count without the record.
 *
 * **A single attempt draws no section header at all.** Most steps are worked
 * once, and a header reading `Attempt 1` above every step on every Job is a
 * word that never varies.
 */
export type StepTimelineRow = {
  id: string;
  /** `Instructed`, `Working`, `Checks`, `Judge`. */
  name: ReactNode;
  /** Where the phase stands, for the mark and its hue. */
  activity: StepActivity;
  /** What the row says while it is closed — `1756 turns · 43m 37s · 36 files`. */
  meta?: ReactNode;
  /** The Drone is in this row now: the mark pulses and the row opens by default. */
  live?: boolean;
  /**
   * What the row opens to. **Absent draws a row that does not open**, which is
   * a phase with nothing recorded in it rather than a control over an empty box.
   */
  body?: ReactNode;
  /**
   * The control on the row's line — `Open the log`, `Open the diff`.
   *
   * **A sibling of the opening control, never inside it**, which is the rule
   * `Chapter` already keeps: a button within a button is one target carrying
   * two meanings, and the outer one wins the press.
   */
  act?: ReactNode;
};

export type StepTimelineAttempt = {
  id: string;
  /** `Attempt 2`. Drawn only where the step was worked more than once. */
  name: ReactNode;
  /** What became of it, in the caller's words — `handed back · 2m 22s`. */
  said?: ReactNode;
  /** The attempt being read. Open on mount; the earlier ones fold. */
  current?: boolean;
  rows: StepTimelineRow[];
};

export type StepTimelineProps = {
  attempts: StepTimelineAttempt[];
  /** The label over the timeline. Absent draws none. */
  label?: ReactNode;
  /**
   * Which row is open, held by the caller. **Present makes the timeline
   * controlled**, so a keyboard map can open a phase by id rather than reaching
   * for the class this component happens to ship. `null` is every row closed.
   */
  openRow?: string | null;
  onOpenRow?: (rowId: string | null) => void;
};

export function StepTimeline({ attempts, label, openRow, onOpenRow }: StepTimelineProps) {
  // Controlled by presence, not by a flag: a caller either holds the value or
  // it does not, and a boolean beside it is a second answer that can disagree.
  const controlled = openRow !== undefined;
  const [held, setHeld] = useState<string | null>(() => whereItIs(attempts));
  const open = controlled ? openRow : held;
  const many = attempts.length > 1;

  function toggle(rowId: string): void {
    const next = open === rowId ? null : rowId;
    if (!controlled) setHeld(next);
    onOpenRow?.(next);
  }

  return (
    <div className="armada-steps">
      {label === undefined ? null : <span className="armada-steps__label">{label}</span>}
      {attempts.map((attempt) => (
        <Attempt key={attempt.id} attempt={attempt} named={many} open={open} onToggle={toggle} />
      ))}
    </div>
  );
}

/** One attempt: its own header where there is more than one, and its phases. */
function Attempt({
  attempt,
  named,
  open,
  onToggle,
}: {
  attempt: StepTimelineAttempt;
  named: boolean;
  open: string | null;
  onToggle: (rowId: string) => void;
}) {
  const [shown, setShown] = useState(attempt.current === true);
  const rows = attempt.rows.map((row) => (
    <Row key={row.id} row={row} open={open === row.id} onToggle={() => onToggle(row.id)} />
  ));
  if (!named) return <div className="armada-steps__rows">{rows}</div>;
  return (
    <section className="armada-steps__attempt">
      <button
        type="button"
        className="armada-steps__attempt-head"
        aria-expanded={shown}
        onClick={() => setShown((was) => !was)}
      >
        {shown ? (
          <ChevronDown className="armada-steps__fold" size={13} strokeWidth={2} aria-hidden />
        ) : (
          <ChevronRight className="armada-steps__fold" size={13} strokeWidth={2} aria-hidden />
        )}
        <span className="armada-steps__attempt-name">{attempt.name}</span>
        {attempt.said === undefined ? null : (
          <span className="armada-steps__said">{attempt.said}</span>
        )}
      </button>
      {/* Hidden rather than unmounted, so a row opened inside an attempt is
          still open when the attempt is folded and opened again. */}
      <div className="armada-steps__rows" hidden={!shown}>
        {rows}
      </div>
    </section>
  );
}

/** One phase of one attempt. */
function Row({
  row,
  open,
  onToggle,
}: {
  row: StepTimelineRow;
  open: boolean;
  onToggle: () => void;
}) {
  const line = (
    <>
      <StepActivityMark activity={row.activity} label={rowLabel(row)} pulsing={row.live} />
      <span className="armada-steps__name">{row.name}</span>
      {row.meta === undefined ? null : <span className="armada-steps__meta">{row.meta}</span>}
    </>
  );
  const act = row.act === undefined ? null : <span className="armada-steps__act">{row.act}</span>;
  if (row.body === undefined) {
    return (
      <div className="armada-steps__row" data-open="false">
        <div className="armada-steps__lid">
          <div className="armada-steps__line">{line}</div>
          {act}
        </div>
      </div>
    );
  }
  return (
    <div className="armada-steps__row" data-open={open}>
      <div className="armada-steps__lid">
        <button
          type="button"
          className="armada-steps__line armada-steps__line--opens"
          aria-expanded={open}
          onClick={onToggle}
        >
          {line}
        </button>
        {act}
      </div>
      <div className="armada-steps__body" hidden={!open}>
        {row.body}
      </div>
    </div>
  );
}

/**
 * The row a timeline opens on.
 *
 * **Where the step is, which is what the strip it replaces said.** The phase a
 * Drone is in wins it; on a step that has finished it is the last phase that
 * did something, because the reason a person opens a finished step is to read
 * what the gates found rather than what it was told at the start.
 *
 * Nothing opens where no row carries a body — four closed lines is a panel
 * reporting that it has nothing, and that is the honest drawing of a step that
 * has not begun.
 */
function whereItIs(attempts: readonly StepTimelineAttempt[]): string | null {
  const rows = (attempts.find((attempt) => attempt.current) ?? attempts[attempts.length - 1])?.rows;
  if (rows === undefined) return null;
  const live = rows.find((row) => row.live === true && row.body !== undefined);
  if (live !== undefined) return live.id;
  const reached = rows.filter((row) => row.body !== undefined && row.activity !== "not_started");
  return reached[reached.length - 1]?.id ?? null;
}

/**
 * The mark's accessible name.
 *
 * **The phase and its state, not the state alone.** Hue and silhouette are the
 * visible channels; a screen reader hearing only `failed` four times down a
 * timeline is told which things went wrong and not which phases they were.
 */
function rowLabel(row: StepTimelineRow): string {
  return typeof row.name === "string" ? row.name : "phase";
}
