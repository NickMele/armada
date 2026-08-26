// Where Bridge says what Fleet is, in the words the design contract settles.
//
// **The bar states a healthy status out loud**, because an empty bar reads the
// same whether Fleet is healthy or dead. The leading dot carries the hue and is
// the one exception to the bar being `--fg-muted` throughout.
//
// Four runtime-file answers, not one message. "Fleet is not running" is three
// different facts underneath — no file, a dead pid, and a pid something else
// now holds — and the third is the one that must never become a socket.

import type { Connection } from "../../shared/bridge";

/** Sentence, detail and hue. The detail is machine-derived and renders in mono. */
export type Statement = {
  headline: string;
  detail: string;
  /** What to do about it, where there is something to do. */
  next: string | null;
  /** A status token stem, spelled `--status-{stem}`. */
  hue: string;
};

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
        hue: "not-started",
      };

    case "not_running":
      return {
        headline: "Fleet is not running",
        detail: absence(connection.absence),
        // Bridge cannot start Fleet, so the sentence says who does.
        next: "Start Fleet. Bridge reconnects on its own.",
        hue: "completed-failed",
      };

    case "runtime_file_refused":
      return {
        headline: "Fleet's runtime file was refused",
        detail: `${connection.fault.why}: ${connection.fault.detail} · ${connection.fault.path}`,
        // Not folded into "not running": the read failed, and calling that a
        // Fleet that is down decides on no evidence.
        next: "Check what wrote the file. Bridge will not connect to a port it names.",
        hue: "escalated",
      };

    case "connecting":
      return {
        headline: "Connecting to Fleet",
        detail: `pid ${connection.fleet.pid} · port ${connection.fleet.port}`,
        next: null,
        hue: "not-started",
      };

    case "unreachable":
      return {
        headline: "Fleet unreachable",
        detail:
          `pid ${connection.fleet.pid} alive on port ${connection.fleet.port}, ` +
          `no answer for ${elapsed(now - connection.sinceMs)} · ${staleness}`,
        // Restarting Fleet is the wrong fix here, so the sentence does not say to.
        next: "Fleet is up and not answering. What is shown below is not live.",
        hue: "awaiting-review",
      };

    case "version_skew":
      return {
        headline: "Fleet speaks a protocol Bridge does not",
        detail: `Fleet ${connection.speaks} · Bridge ${connection.expected}`,
        next: "Fleet and Bridge ship as a pair. Update both to the same commit.",
        hue: "escalated",
      };

    case "connected":
      return {
        headline: "Fleet running",
        detail: `pid ${connection.fleet.pid} · port ${connection.fleet.port} · ${staleness}`,
        next: null,
        hue: "completed-success",
      };
  }
}

export function FleetBar({ statement }: { statement: Statement }) {
  return (
    <footer className="flex h-status-bar shrink-0 items-center gap-2 border-t border-border-subtle bg-bg-raised px-4 text-2xs text-fg-muted">
      <span
        className="h-dot w-dot shrink-0 rounded-full"
        style={{ background: `var(--status-${statement.hue})` }}
        aria-hidden
      />
      <span>{statement.headline}</span>
      {statement.detail === "" ? null : (
        <>
          <span className="text-fg-subtle">·</span>
          <span className="mono">{statement.detail}</span>
        </>
      )}
    </footer>
  );
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
