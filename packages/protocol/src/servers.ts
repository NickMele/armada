// A Command that stays running, held by Fleet — `crates/ipc/src/servers.rs`.
// Since protocol 10.9.
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

/** One instance. `list_servers`' row, and every `server.*` event's payload. */
export type ServerState = {
  /** What `stop_server` and `observe_server` take. */
  id: string;
  name: string;
  /** Absent: started with no Job, in the main checkout. */
  job_id?: string;
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

/** `POST /servers/start`'s body. A name, never a command line or a port. */
export type StartServer = {
  name: string;
  /** Absent: the main checkout, on its span. */
  job_id?: string;
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
