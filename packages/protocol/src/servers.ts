// A Command that stays running, held by Fleet — `crates/ipc/src/servers.rs`.
// Since protocol 10.10.
//
// **Lifecycle on `/events`, output on a socket of its own**, the run socket's
// reason one subject over. The header rules in `events.ts` hold: hand-written,
// and every closed set left as `string`.

import type { ProtocolVersion } from "./version";

/** An address a server offers, resolved against the span it runs under. */
export type ServerLink = {
  url: string;
  /** The button's label. Absent: draw the URL. */
  name?: string;
};

/** A declared port a server's fields name, and the number it resolved to. */
export type ServerPort = { name: string; port: number };

/**
 * Which checkout a server serves, and whether what it serves is still what
 * that checkout stands at. Since protocol 18.1.
 *
 * A name and a port are not enough: two checkouts of one repository declare
 * the same server under the same name, and the address answers either way.
 */
export type ServerCheckout = {
  /** The checkout it runs in, absolute: a Job's worktree, or a main checkout. */
  path: string;
  /** The branch checked out there. Absent for a main checkout. */
  branch?: string;
  /** The commit the checkout stood at when the server started. */
  commit?: string;
  /**
   * How many commits have landed in that checkout since. **Absent is not
   * zero** — zero is nothing having landed, absent is Fleet never having
   * been told.
   */
  behind?: number;
};

/** One instance. `list_servers`' row, and every `server.*` event's payload. */
export type ServerState = {
  /** What `stop_server` and `observe_server` take. */
  id: string;
  name: string;
  /** Absent: started with no Job, in the main checkout. */
  job_id?: string;
  /**
   * The repository it runs in, by its Manifest — a Job's own, or the main
   * checkout's. Absent only from a Fleet that predates it. Since protocol 13.17.
   */
  manifest_id?: string;
  /** Which checkout answers on this server's address. Since protocol 18.1. */
  checkout: ServerCheckout;
  /** `starting`, `serving` or `exited`. Only `serving` carries a live address. */
  phase: string;
  /** The `serve` line as it ran, `${port.NAME}` resolved. */
  serve: string;
  /** In the order its fields name them. `localhost` and the first is where it is. */
  ports: ServerPort[];
  links: ServerLink[];
  /** `person` or `drone`. */
  started_by: string;
  started_at: string;
  /** Uptime counts from here; nothing ticks on the wire. */
  serving_since?: string;
  ended_at?: string;
  exit_code?: number;
  /** How it ended. A server that exits on its own has failed, whatever its code. */
  ended?: string;
  /** Something stopped it. `false` on an `exited` server: it stopped on its own. */
  stopped: boolean;
  /** The log, relative to `ManifestSummary.records_root`. */
  log: string;
};

/** `GET /servers` — every server held, and the last of each that ended. */
export type ServerList = { servers: ServerState[] };

/**
 * `POST /servers/start`'s body. A name, never a command line or a port. Which
 * repository is `?manifest_id=`; absent, the one Fleet started in.
 */
export type StartServer = {
  name: string;
  /** Absent: a checkout, on its span. */
  job_id?: string;
  /**
   * A checkout of that repository to serve, by its path — a worktree cut by
   * hand to look at another branch. Absent: the main checkout. Ignored where
   * `job_id` names a Job. Since protocol 18.2.
   */
  checkout?: string;
};

/** `POST /servers/stop`'s body. */
export type NamedServer = { id: string };

/** One declared server, as the run sheet lists it beside the Commands. */
export type ServerEntry = {
  name: string;
  run?: string;
  /** The `serve` line, as declared. */
  serve: string;
  ready?: string;
  /** As declared, `${port.NAME}` unresolved. */
  links: ServerLink[];
  destructive: boolean;
  /** This Job's instance: running, or the last that ended. */
  instance?: ServerState;
};

/** One message on `GET /servers/:server_id/observe` — the run socket's four. */
export type ServerMessage =
  | ({ message: "opened" } & ServerOpened)
  | ({ message: "lines" } & { lines: string[] })
  | ({ message: "missed" } & { dropped: number })
  | ({ message: "closed" } & { because: string });

export type ServerOpened = {
  protocol_version: ProtocolVersion;
  id: string;
  name: string;
  job_id?: string;
  path: string;
  /** Still running when this opened. `false`: the history is all of it. */
  live: boolean;
  skipped: number;
};
