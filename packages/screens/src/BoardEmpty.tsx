// What the Board says when it draws no row. Out of `Jobs.tsx`, which reached the 500 lines the gate
// warns at when a fourth case joined these three: Fleet serving no repository yet.

import { actionOf, BoardEmptyState, Button, Kbd } from "@armada/components";

import { NOTHING_SERVED } from "./locate-reads";

/**
 * Overview's null result: a Fleet that is up, serves a repository, and holds no Job. #1262.
 *
 * **A card with the act on it, not the Board's line pointing away.** "Propose one above" sent a
 * person across the window to the title row's Dispatch; here the same composer is one press, and
 * the Primary is the view's one solid accent — the title row's stays Tonal. **Only this case.**
 * The fault and fresh-install states stay `BoardEmpty`'s own, and the fresh install offers no
 * Dispatch because there is no repository to name.
 */
export function OverviewEmpty({ onCompose }: { onCompose: () => void }) {
  // Verb and key from the registry's own act, never retyped: `n` is what the key map answers.
  const dispatch = actionOf("new_job");
  return (
    <BoardEmptyState
      card
      quiet
      lead="No jobs."
      action={
        <Button variant="primary" onClick={onCompose}>
          {dispatch.verb}
          <Kbd aria-hidden>{dispatch.shortcut}</Kbd>
        </Button>
      }
    >
      Propose one.
    </BoardEmptyState>
  );
}

export function BoardEmpty({
  disconnected,
  why,
  suspended,
  nothingServed,
  onClear,
}: {
  disconnected: string | null;
  /** What emptied the list, where a control did. */
  why: string | null;
  suspended: boolean;
  /** Fleet has listed no repository, so there is nothing a Job could be proposed against. */
  nothingServed: boolean;
  /** Clears whichever control emptied it. */
  onClear: () => void;
}) {
  // The three empty states differ because the three situations do: a
  // Fleet that is up with no work is a null result, one that is not
  // running is a fault Bridge cannot fix, and a filter that emptied
  // the list is neither — it is a control saying so.
  if (disconnected !== null) {
    return (
      <BoardEmptyState command="armada fleet start" note={disconnected}>
        Fleet is not connected, so there is nothing to show.
      </BoardEmptyState>
    );
  }
  if (why !== null) {
    // **The control offered is the one that emptied the list.** Only
    // one of the two can have, because a search suspends the tab — so
    // clearing the search restores whatever tab was set rather than
    // discarding it, which is the whole reason suspending won over
    // resetting. A single "Show every job" here would have spent the
    // filter at the last moment it could have been kept.
    return (
      <BoardEmptyState
        quiet
        action={
          <Button variant="ghost" size="sm" onClick={onClear}>
            {suspended ? "Clear the search" : "Show every job"}
          </Button>
        }
      >
        {why}
      </BoardEmptyState>
    );
  }
  // A fresh install: "Propose one above" would send a person to a composer with nothing to name.
  if (nothingServed) return <BoardEmptyState quiet>{`${NOTHING_SERVED.title}. ${NOTHING_SERVED.next}`}</BoardEmptyState>;
  return <BoardEmptyState quiet>No jobs. Propose one above.</BoardEmptyState>;
}
