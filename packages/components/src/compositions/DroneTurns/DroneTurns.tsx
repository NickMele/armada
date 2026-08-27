import { Fragment, useState, type ReactNode } from "react";
import { ChevronDown, ChevronRight, CircleDot } from "lucide-react";
import { Button } from "../../primitives/Button/Button";

/**
 * Drone turns — one Drone's transcript, read while it is still being written.
 *
 * **Read-only, and it must not read as Pilot.** Observing changes nothing about
 * the Job: no status moves, no transition is recorded, the Drone is never told.
 * So nothing in this component takes a control, and a row carries no act —
 * `docs/concepts/observe.md` is the table that separates the two. The one
 * control here reveals rows this pane already holds.
 *
 * **A call and its answer are one row.** They arrive as two events with the
 * tool running in the gap between them, and two rows would separate a command
 * from its output by everything that happened while it ran. The join is on the
 * call id and it happens here rather than in Fleet, where holding a call open
 * to wait for its result would be unbounded buffering in the loop that advances
 * the Job.
 *
 * **A row kind is the wire's own word, in mono.** No vocabulary in the
 * repository carries a verb, a glyph or a hue per turn kind — `Saw` is the
 * wire's enum and has no `enum-verbs.toml` rows — so the spelling renders
 * rather than copy invented here. Reported.
 */
export type DroneTurn = {
  /** Stable across re-renders. Rows arrive in order and nothing reorders them. */
  id: string;
  /** When Fleet's line loop saw it, not when it reached the disk. */
  at: string;
  /** The wire's kind: `called`, `said`, `refused`, `started`, and the rest. */
  kind: string;
  /** The machine value the row is about — a tool, a session, an unread line. */
  subject?: string;
  /**
   * What the call did: the command run, the path read, the pattern searched.
   * Rendered where the wire carries it and **never derived from the tool
   * name** — a tool name alone is what made twenty-two rows read alike.
   */
  detail?: ReactNode;
  /** `detail` was cut short upstream. Rendered as cut, never as the whole value. */
  truncated?: boolean;
  /**
   * What came back, where this row is a call joined to its answer. Absent on a
   * call still running, which the row says in words rather than leaving blank.
   */
  answer?: ReactNode;
  /** Prose: the Drone's own text, or the harness's wording for a refusal. */
  said?: ReactNode;
  /**
   * The row is the Drone thinking rather than something it did. Consecutive
   * quiet rows collapse to one line, expandable, keeping the count.
   *
   * Measured on one real transcript: 106 of 149 rows. Naming them by the
   * decoder's failure to place them described the plumbing; from the reader's
   * side it is a model working, which is what the collapsed line says.
   */
  quiet?: boolean;
};

export type DroneTurnsProps = {
  /** In the order Fleet sent them. History first, then live rows. */
  turns: DroneTurn[];
  /**
   * What the pane says with no rows at all. A Job that was never dispatched is
   * ordinary rather than an error, and the sentence has to say which.
   */
  emptyNote: string;
  /**
   * Whether a Drone is writing. Decides whether the trailing collapsed run
   * says so and its mark moves — a finished transcript showing a live mark on
   * a gap in its middle would claim work that stopped.
   */
  live?: boolean;
};

export function DroneTurns({ turns, emptyNote, live = false }: DroneTurnsProps) {
  const [open, setOpen] = useState<ReadonlySet<string>>(() => new Set());

  if (turns.length === 0) {
    return (
      <p className="armada-turns__empty" role="note">
        {emptyNote}
      </p>
    );
  }

  const entries = runs(turns);
  return (
    <ol className="armada-turns">
      {entries.map((entry, at) =>
        entry.of === "turn" ? (
          <Row key={entry.turn.id} turn={entry.turn} />
        ) : (
          <QuietRun
            key={entry.turns[0].id}
            turns={entry.turns}
            // Only the last run can still be happening: a run with rows after
            // it already ended, and the contract allows one animated mark per
            // screen anyway.
            working={live && at === entries.length - 1}
            open={open.has(entry.turns[0].id)}
            onToggle={() => setOpen(toggled(open, entry.turns[0].id))}
          />
        ),
      )}
    </ol>
  );
}

type Entry = { of: "turn"; turn: DroneTurn } | { of: "quiet"; turns: DroneTurn[] };

/**
 * The rows, with consecutive quiet ones gathered.
 *
 * **A run of one is still a run.** Left alone it renders the decoder's own
 * words for a turn it could not place, which is the reading this collapse
 * exists to remove, and one line that reads like its neighbours beats one that
 * does not.
 */
function runs(turns: DroneTurn[]): Entry[] {
  const entries: Entry[] = [];
  for (const turn of turns) {
    const last = entries[entries.length - 1];
    if (turn.quiet !== true) {
      entries.push({ of: "turn", turn });
      continue;
    }
    if (last !== undefined && last.of === "quiet") {
      last.turns.push(turn);
      continue;
    }
    entries.push({ of: "quiet", turns: [turn] });
  }
  return entries;
}

function toggled(open: ReadonlySet<string>, id: string): ReadonlySet<string> {
  const next = new Set(open);
  if (!next.delete(id)) next.add(id);
  return next;
}

/** The mark is 12px at strokeWidth 2, the step mark's geometry one level down. */
const MARK = 12;
const MARK_STROKE = 2;
const CARET = 16;

type QuietRunProps = {
  turns: DroneTurn[];
  working: boolean;
  open: boolean;
  onToggle: () => void;
};

/**
 * A run of quiet rows as one line, and the rows themselves when it is opened.
 *
 * **The rows are never dropped, only folded.** This pane already tells a viewer
 * when the backfill skipped rows and when a slow viewer lost some; hiding rows
 * with no trace would contradict both, and a pane that cannot be trusted to
 * hold everything cannot answer what the Drone did.
 */
function QuietRun({ turns, working, open, onToggle }: QuietRunProps) {
  const head = turns[0];
  // Only while they exist. `aria-controls` naming ids that are not in the
  // document is an invalid value, not an empty one.
  const held = open ? turns.map((turn) => rowId(turn)).join(" ") : undefined;
  return (
    <Fragment>
      <li className="armada-turns__turn" data-quiet data-open={open || undefined}>
        <span className="armada-turns__at">{head.at}</span>
        <span className="armada-turns__mark" data-working={working || undefined}>
          <CircleDot size={MARK} strokeWidth={MARK_STROKE} aria-hidden />
        </span>
        <span className="armada-turns__quiet-body">
          {/* Live only. A finished transcript is a record, and a record does
              not narrate: once nothing is happening the count is the whole
              fact and the still mark already says the run ended. */}
          {working ? <span className="armada-turns__working">{"Working"}</span> : null}
          <span className="armada-turns__count">{counted(turns.length)}</span>
          <Button
            variant="ghost"
            size="sm"
            aria-expanded={open}
            aria-controls={held}
            onClick={onToggle}
          >
            {open ? (
              <ChevronDown size={CARET} strokeWidth={MARK_STROKE} aria-hidden />
            ) : (
              <ChevronRight size={CARET} strokeWidth={MARK_STROKE} aria-hidden />
            )}
            {open ? "Hide details" : "Show details"}
          </Button>
        </span>
      </li>
      {open ? turns.map((turn) => <Row key={turn.id} turn={turn} nested />) : null}
    </Fragment>
  );
}

function Row({ turn, nested = false }: { turn: DroneTurn; nested?: boolean }) {
  return (
    <li className="armada-turns__turn" id={rowId(turn)} data-nested={nested || undefined}>
      <span className="armada-turns__at">{turn.at}</span>
      <span className="armada-turns__kind">{turn.kind}</span>
      <span className="armada-turns__body">
        {turn.subject === undefined && turn.detail === undefined ? null : (
          <span className="armada-turns__head">
            {turn.subject === undefined ? null : (
              <span className="armada-turns__subject">{turn.subject}</span>
            )}
            {turn.detail === undefined ? null : (
              <span className="armada-turns__detail">
                {turn.detail}
                {turn.truncated === true ? <Cut /> : null}
              </span>
            )}
          </span>
        )}
        {turn.answer === undefined ? null : (
          <span className="armada-turns__answer">{turn.answer}</span>
        )}
        {turn.said === undefined ? null : <span className="armada-turns__said">{turn.said}</span>}
      </span>
    </li>
  );
}

/**
 * A value the wire cut short. **Never a tooltip**, which the contract gives
 * anything truncated in a row: there is no fuller value anywhere to put in one,
 * so the ellipsis is the whole fact and the note says it in words.
 */
function Cut() {
  return (
    <span className="armada-turns__cut">
      {"…"}
      <span className="armada-turns__cut-note">{" cut short"}</span>
    </span>
  );
}

function rowId(turn: DroneTurn): string {
  return `armada-turn-${turn.id}`;
}

/** `70 turns`, and `1 turn`. */
function counted(rows: number): string {
  return rows === 1 ? "1 turn" : `${rows} turns`;
}
