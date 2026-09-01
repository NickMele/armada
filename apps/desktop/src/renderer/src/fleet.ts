// What Fleet is, in the words the design contract settles. The status bar
// draws them; this decides them.
//
// **A healthy status is stated out loud**, because an empty bar reads the same
// whether Fleet is healthy or dead.
//
// Four runtime-file answers, not one message. "Fleet is not running" is three
// different facts underneath — no file, a dead pid, and a pid something else
// now holds — and the third is the one that must never become a socket.
//
// This was `FleetBar.tsx` and drew the bar itself. The bar is
// `Compositions/Status bar` now, so what is left is the reading.

import type { Connection } from "@armada/protocol";
import { PROTOCOL_VERSION } from "@armada/protocol";
import { spoken } from "@armada/protocol";

/** Sentence, detail and hue. The detail is machine-derived and renders in mono. */
export type Statement = {
  headline: string;
  detail: string;
  /** What to do about it, where there is something to do. */
  next: string | null;
};

// **No hue here any more.** This carried a status token stem per state, and
// the bar drew a dot in it. The contract's bar names three states and the two
// readings that are neither — a refused runtime file, a protocol Bridge does
// not speak — took a fourth hue that the contract does not grant. They keep
// the neutral dot; the sentence names them, and the failure notice on the
// board carries the whole reading.

export function elapsed(ms: number): string {
  const seconds = Math.max(0, Math.round(ms / 1000));
  if (seconds < 60) return `${seconds}s`;
  const minutes = Math.round(seconds / 60);
  return minutes < 60 ? `${minutes}m` : `${Math.round(minutes / 60)}h`;
}

export function statementOf(connection: Connection, now: number, readAt: number | null): Statement {
  const staleness = readAt === null ? "nothing read yet" : `last read ${elapsed(now - readAt)} ago`;

  switch (connection.state) {
    case "reading":
      return {
        headline: "Reading Fleet's runtime file",
        detail: "",
        next: null,
      };

    case "not_running":
      return {
        headline: "Fleet is not running",
        detail: absence(connection.absence),
        // Bridge cannot start Fleet, so the sentence says who does.
        next: "Start Fleet. Bridge reconnects on its own.",
      };

    case "runtime_file_refused":
      return {
        headline: "Fleet's runtime file was refused",
        detail: `${connection.fault.why}: ${connection.fault.detail} · ${connection.fault.path}`,
        // Not folded into "not running": the read failed, and calling that a
        // Fleet that is down decides on no evidence.
        next: "Check what wrote the file. Bridge will not connect to a port it names.",
      };

    case "connecting":
      return {
        headline: "Connecting to Fleet",
        detail: `pid ${connection.fleet.pid} · port ${connection.fleet.port}`,
        next: null,
      };

    case "unreachable":
      return {
        headline: "Fleet unreachable",
        detail:
          `pid ${connection.fleet.pid} alive on port ${connection.fleet.port}, ` +
          `no answer for ${elapsed(now - connection.sinceMs)} · ${staleness}`,
        // Restarting Fleet is the wrong fix here, so the sentence does not say to.
        next: "Fleet is up and not answering. What is shown below is not live.",
      };

    case "version_skew": {
      const versions = `Fleet ${spoken(connection.speaks)} · Bridge ${spoken(connection.expected)}`;
      // Two refusals, two sentences. A different protocol needs both sides
      // moved; a Fleet merely behind needs only the daemon restarted, and
      // telling somebody to rebuild both would be telling them to do more than
      // the situation asks.
      return connection.why === "incompatible"
        ? {
            headline: "Fleet speaks a protocol Bridge does not",
            detail: versions,
            next: "Fleet and Bridge ship as a pair. Update both to the same commit.",
          }
        : {
            headline: "Fleet is older than Bridge",
            detail: versions,
            next: "Bridge reads fields this Fleet is too old to send. Restart Fleet when no Job is running.",
          };
    }

    case "connected":
      // **No "last read" here, and that is the point.** A healthy connection
      // folds `job.created`, `job.state_changed` and `job.step_advanced` as
      // they arrive, so the Board is current by construction and an age beside
      // it says the opposite of what is true. The age stays on `unreachable`,
      // which is the state where how old the reading is is the whole fact.
      return {
        headline: "Fleet running",
        detail:
          `pid ${connection.fleet.pid} · port ${connection.fleet.port}` +
          (connection.skew === "fleet_ahead"
            ? ` · Fleet ${spoken(connection.fleet.protocolVersion)}, Bridge ${spoken(PROTOCOL_VERSION)}`
            : ""),
        // **The one `next` on a healthy connection, and it is not a fault.** A
        // minor bump is additive only, so everything drawn here is current and
        // correct and the only fact is that Fleet knows more than this Bridge
        // can ask about. Said out loud because the alternative is a person
        // meeting a feature that exists and not knowing why they cannot see it
        // — and said in the status bar rather than as a failure notice, which
        // would tell them something is broken when nothing is.
        next:
          connection.skew === "fleet_ahead"
            ? "Fleet is newer than Bridge. Nothing here is stale; update Bridge to reach what it adds."
            : null,
      };
  }
}

/** The three ways a runtime file says Fleet is not running. */
function absence(why: NotRunning): string {
  switch (why.why) {
    case "no_runtime_file":
      return `no runtime file at ${why.path}`;
    case "pid_dead":
      return `pid ${why.pid} is held by nothing · Fleet exited without cleaning up`;
    case "pid_held_by_another":
      // The row a bare liveness check gets wrong.
      return (
        `pid ${why.pid} is held by a process that started ${why.holder}, ` +
        `not ${why.wrote} · its port is not Fleet's`
      );
  }
}

type NotRunning = Extract<Connection, { state: "not_running" }>["absence"];
