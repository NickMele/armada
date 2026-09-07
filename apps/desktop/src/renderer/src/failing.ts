// Which failure the window is showing, and which one leaves it when somebody
// asks for the machine's account of it.
//
// # Nothing here fails silently
//
// Three failures, and they stay three. Fleet unreachable says which of the four
// answers the runtime file gave, a command that failed says whether Fleet
// refused it or never answered, and a throw no boundary saw names itself. Each
// points at its log, and the one that carries a run id — a refusal Fleet minted
// — says that the id names Fleet's run rather than that failure. **Nothing here
// mints one.** An id from a process that writes no log line joins to nothing,
// and a labelled blank is worse than an absent row.
//
// # Why the order is an order
//
// `Copy debug info` copies one failure, so one has to be chosen, and the choice
// is which explains the others. Fleet being unreachable outranks a throw in a
// handler: it is the one that explains every other symptom on screen. A command
// that failed sits between them — more specific than a stray throw, and still
// explained by an unreachable Fleet where there is one.
//
// The board's own unreadable rows are not in this order and never were. Each is
// drawn beside the row it is about, and there can be several at once.

import type { BridgeIdentity, Connection, Outcome } from "@armada/protocol";
import type { Failure, Statement, Uncaught } from "@armada/shell";
import { fleetFailure, refusalFailure, transportFailure, uncaughtFailure } from "@armada/shell";
import { statementOf } from "@armada/shell";

/** What the window has been published, as far as a failure is concerned. */
export type Published = {
  connection: Connection;
  /** Both protocol versions and Bridge's log, quoted on every payload. */
  bridge: BridgeIdentity;
  /** When Fleet was last read, for the staleness the status bar says. */
  readAt: number | null;
  /** What the last command answered, failure or not. */
  outcome: Outcome | null;
  /** A throw or a rejection no boundary could have caught. */
  uncaught: Uncaught | null;
  /** The one clock every elapsed figure in the window is drawn from. */
  now: number;
};

/** The failures on screen, and the one that would be sent. */
export type Failing = {
  /** What the connection is, in the app's voice, whether or not it is broken. */
  statement: Statement;
  /** Fleet, where the one connection is not a connection. */
  fleet: Failure | null;
  /** The last command, where it failed rather than refused to send. */
  commandFailure: Failure | null;
  /** The one `Copy debug info` copies, or none. */
  failing: Failure | null;
};

export function failingIn(published: Published): Failing {
  const { connection, bridge, readAt, outcome, uncaught, now } = published;
  const statement = statementOf(connection, now, readAt);
  const fleet = fleetFailure(connection, statement, bridge, now);
  // What the last command answered, where the answer was a failure rather than
  // guidance. **Two arms and not one**: a refusal is Fleet declining with an
  // envelope, and a transport failure is a command it did not answer at all —
  // which used to be a single line of copy with no code and nothing to copy.
  // Everything else `Outcome` carries is the form saying what it will not send,
  // which is guidance and takes the `Alert` the surface draws.
  const commandFailure =
    outcome === null || outcome.ok
      ? null
      : outcome.why === "refused"
        ? refusalFailure(outcome.error, bridge)
        : outcome.why === "transport"
          ? transportFailure(outcome, bridge)
          : null;

  return {
    statement,
    fleet,
    commandFailure,
    failing:
      fleet ?? commandFailure ?? (uncaught === null ? null : uncaughtFailure(uncaught, bridge)),
  };
}
