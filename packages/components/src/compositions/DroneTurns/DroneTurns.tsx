import type { ReactNode } from "react";

/**
 * Drone turns — one Drone's transcript, read while it is still being written.
 *
 * **Read-only, and it must not read as Pilot.** Observing changes nothing about
 * the Job: no status moves, no transition is recorded, the Drone is never told.
 * So nothing in this component takes a control, and a row carries no act —
 * `docs/concepts/observe.md` is the table that separates the two.
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
   * What came back, where this row is a call joined to its answer. Absent on a
   * call still running, which the row says in words rather than leaving blank.
   */
  answer?: ReactNode;
  /** Prose: the Drone's own text, or the harness's wording for a refusal. */
  said?: ReactNode;
};

export type DroneTurnsProps = {
  /** In the order Fleet sent them. History first, then live rows. */
  turns: DroneTurn[];
  /**
   * What the pane says with no rows at all. A Job that was never dispatched is
   * ordinary rather than an error, and the sentence has to say which.
   */
  emptyNote: string;
};

export function DroneTurns({ turns, emptyNote }: DroneTurnsProps) {
  if (turns.length === 0) {
    return (
      <p className="armada-turns__empty" role="note">
        {emptyNote}
      </p>
    );
  }

  return (
    <ol className="armada-turns">
      {turns.map((turn) => (
        <li className="armada-turns__turn" key={turn.id}>
          <span className="armada-turns__at">{turn.at}</span>
          <span className="armada-turns__kind">{turn.kind}</span>
          <span className="armada-turns__body">
            {turn.subject === undefined ? null : (
              <span className="armada-turns__subject">{turn.subject}</span>
            )}
            {turn.answer === undefined ? null : (
              <span className="armada-turns__answer">{turn.answer}</span>
            )}
            {turn.said === undefined ? null : (
              <span className="armada-turns__said">{turn.said}</span>
            )}
          </span>
        </li>
      ))}
    </ol>
  );
}
