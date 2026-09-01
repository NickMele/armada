// Where Fleet is, and why it is not answering.
//
// **Not about Electron.** A runtime file at a path, a pid, a port and a
// protocol version are what any client of Fleet reads to find it, so they sit
// with the wire rather than with the process that happens to read them here.
// How the answer reaches a renderer is `apps/desktop`'s business and is not
// described here.

import { connects, skew } from "./version";
import type { ProtocolVersion, Skew } from "./version";
import { PROTOCOL_VERSION } from "./generated/protocol-version";

/** Fleet, as its runtime file names it. Loopback plus `port` is the address. */
export type FleetIdentity = {
  /** Both numbers. Which one differs from Bridge's decides what happens. */
  protocolVersion: ProtocolVersion;
  pid: number;
  port: number;
  /** `ps -o lstart=` as it read when Fleet published the file. */
  startedAt: string;
};

/**
 * Why the runtime file does not describe a live Fleet.
 *
 * Three, not one. Bridge renders the first two as "Fleet is not running" and
 * says which under it, because the third — a pid something else now holds — is
 * the case a bare liveness check gets wrong, and the consequence is a socket
 * opened against a port an unrelated program owns.
 */
export type Absence =
  | { why: "no_runtime_file"; path: string }
  | { why: "pid_dead"; path: string; pid: number }
  | { why: "pid_held_by_another"; path: string; pid: number; wrote: string; holder: string };

/**
 * Why the runtime file could not be read at all. **None of these is "not
 * running"** — that is a fact about the world, and folding a failed read into
 * it tells a person Fleet is down on no evidence.
 */
export type RuntimeFault =
  | { why: "unreadable"; path: string; detail: string }
  | { why: "undecodable"; path: string; detail: string }
  | { why: "probe_failed"; path: string; pid: number; detail: string };

/** Where Bridge's one connection is. */
export type Connection =
  | { state: "reading" }
  | { state: "not_running"; absence: Absence }
  | { state: "runtime_file_refused"; fault: RuntimeFault }
  | { state: "connecting"; fleet: FleetIdentity }
  /** The pid checks out and the socket does not answer. A different thing to do. */
  | { state: "unreachable"; fleet: FleetIdentity; detail: string; sinceMs: number }
  /**
   * Fleet speaks a protocol Bridge will not connect over. Two readings, and
   * they need different sentences: `incompatible` is a different protocol,
   * `fleet_behind` is the same protocol missing additions Bridge now expects.
   */
  | {
      state: "version_skew";
      fleet: FleetIdentity;
      why: Extract<Skew, "fleet_behind" | "incompatible">;
      speaks: ProtocolVersion;
      expected: ProtocolVersion;
    }
  /**
   * Connected, and `skew` says whether there is a caveat on it.
   *
   * `fleet_ahead` is **not a failure** and must not render as one: everything
   * Bridge draws is current and correct, and the only fact is that Fleet knows
   * things this Bridge was built too early to ask about. Decided here, once, so
   * no surface re-derives it.
   */
  | {
      state: "connected";
      fleet: FleetIdentity;
      cursor: number;
      skew: Extract<Skew, "same" | "fleet_ahead">;
    };

/**
 * A live connection, saying which of the two readings it is.
 *
 * Here rather than at the call site so nothing can publish `connected` without
 * deciding whether Fleet is ahead — and narrowed rather than cast, because the
 * two readings that refuse never reach a socket.
 */
export function connectedTo(fleet: FleetIdentity, cursor: number): Connection {
  const reading = skew({ fleet: fleet.protocolVersion, bridge: PROTOCOL_VERSION });
  return { state: "connected", fleet, cursor, skew: connects(reading) ? reading : "same" };
}

/* What the side holding the connection knows about its own session, which the
 * side rendering a failure cannot derive.
 *
 * **Here rather than with the app that fills it.** Both a host process and the
 * surface that draws its failures need this type, and they sit at opposite ends
 * of the layering — the only place both can reach is the bottom. It is the
 * loosest thing in this package: an audit path is a local file, not something
 * Fleet ever sends. It travels because a shared vocabulary is what this package
 * is for, not because a socket carries it. */
/**
 * What every Bridge failure carries that is not about the failure — where the
 * machine log is, and which Fleet is on the other end of the one connection.
 *
 * **No `run_id`, and none is minted.** The envelope makes `run_id` the one id an
 * emitter mints for itself, but nothing in Bridge writes a log line yet, so an
 * id minted here would join to nothing and would read on screen as though it
 * identified the failure. The only real one is the one a `WireError` carries,
 * and that names Fleet's run rather than any single failure.
 *
 * Both fields are facts main holds and the renderer cannot derive — a home
 * directory it cannot resolve, and a connection it does not own — so both are
 * published rather than guessed at.
 */
export type BridgeIdentity = {
  /** The machine log. `null` where HOME is not set and no path resolves. */
  auditPath: string | null;
  /**
   * The protocol Fleet speaks, written `5.2`, as the runtime file said it.
   *
   * **Here rather than at each failure, because four of the five failures are
   * handed no connection.** A refusal is the case that made it worth fixing:
   * Fleet answered it, so Fleet's version is the first thing a reader of the
   * payload wants, and it was the one payload guaranteed to omit it. Derived
   * where the connection is published, so nothing re-derives it per failure.
   *
   * `null` before a runtime file has been read and believed, and again the
   * moment the connection is one of the states that never got a version —
   * which is a fact rather than a gap, and the tail omits the row.
   */
  fleetProtocol: string | null;
};
