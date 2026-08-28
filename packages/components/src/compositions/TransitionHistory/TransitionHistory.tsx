import type { ReactNode } from "react";

/**
 * Transition history — every move one Job made, in order.
 *
 * **Rendered, never replayed.** `crates/store/src/fold.rs` owns the machine:
 * every recorded move goes back through `Job::transition` there, and a history
 * the machine would not admit fails to fold rather than producing a Job no
 * legal sequence could reach. Nothing here does that. A second machine would
 * agree with the first only until one of them changed.
 *
 * **`seq` is the order, never `at`.** The instant is injected rather than read
 * from a clock, so two moves inside one millisecond carry the same one. The key
 * is drawn as well as obeyed — it is what a person joins to when they go and
 * look at the row itself.
 *
 * **Not the Drone's turns.** A transcript renders what the Drone said; this
 * renders what Armada did. A Job that went wrong usually needs both, side by
 * side rather than blended, so the two stay two surfaces.
 *
 * **Neutral, all the way down.** Hue below Job level exists only where
 * `tokens/status.css` declares it, and it declares no history row — twelve
 * statuses hued down a list would also read as twelve verdicts. The wire's own
 * word for what moved carries the scan instead, in the kind column.
 */
export type TransitionMove = {
  /** The key the log assigned. Monotonic, never reused, and what orders this. */
  seq: number;
  /** When it was recorded. Read, never sorted on. */
  at: string;
  /**
   * What moved: `status`, `step` or `drone`, in the wire's own word.
   *
   * **The spelling renders.** No vocabulary in the repository carries a verb,
   * a glyph or a hue per movement kind, so a word chosen here would be a second
   * vocabulary. Reported.
   */
  kind: string;
  /** Which one moved, where the row names one: a step id, a Drone id. */
  subject?: string;
  /** The move itself — `running → escalated`. Composed from two served values. */
  moved: ReactNode;
  /** The reason the transition stored, or the trigger that stopped a step. */
  why?: ReactNode;
  /** Who caused it: `human`, `fleet` or `drone`. Three ways, and no fourth. */
  actor: string;
};

export type TransitionHistoryProps = {
  /** Every recorded move, oldest first. */
  moves: TransitionMove[];
  /**
   * What the region says with no moves at all. **Empty is a real answer** — a
   * Job created and not yet moved has no events, because creation is not a
   * transition and no row describes it.
   */
  emptyNote: string;
  /** What the list is, and what it is not. Under it, never inside it. */
  note?: ReactNode;
};

export function TransitionHistory({ moves, emptyNote, note }: TransitionHistoryProps) {
  if (moves.length === 0) {
    return (
      <p className="armada-history__empty" role="note">
        {emptyNote}
      </p>
    );
  }

  return (
    <div className="armada-history">
      <ol className="armada-history__moves">
        {moves.map((move) => (
          <li className="armada-history__move" key={move.seq}>
            <span className="armada-history__seq">{move.seq}</span>
            <span className="armada-history__at">{move.at}</span>
            <span className="armada-history__kind">{move.kind}</span>
            <span className="armada-history__body">
              <span className="armada-history__head">
                {move.subject === undefined ? null : (
                  <span className="armada-history__subject">{move.subject}</span>
                )}
                <span className="armada-history__moved">{move.moved}</span>
              </span>
              {move.why === undefined ? null : (
                <span className="armada-history__why">{move.why}</span>
              )}
            </span>
            <span className="armada-history__actor">{move.actor}</span>
          </li>
        ))}
      </ol>
      {note === undefined ? null : <p className="armada-history__note">{note}</p>}
    </div>
  );
}
