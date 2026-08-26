// What the three processes agree on: the state main publishes, the operations
// the renderer may initiate, and the channel names those two travel over.
//
// Types only, plus the channel constants. Nothing here runs in more than one
// process — the preload is a wire and not an import path, and this file is the
// shape of what crosses it.

import type { JobSummary, UnreadableJob, WireError } from "./protocol";

/** Fleet, as its runtime file names it. Loopback plus `port` is the address. */
export type FleetIdentity = {
  protocolVersion: number;
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
  | { state: "version_skew"; fleet: FleetIdentity; speaks: number; expected: number }
  | { state: "connected"; fleet: FleetIdentity; cursor: number };

/** Everything the renderer draws, published by main and never assembled twice. */
export type BridgeState = {
  connection: Connection;
  jobs: JobSummary[];
  /** Rows the store refused. Shown, never merged into `jobs` as a placeholder. */
  unreadable: UnreadableJob[];
  /** Events Fleet dropped before Bridge saw them, since the window opened. */
  missed: number;
  /** When the Jobs above were last current, in epoch milliseconds. */
  readAt: number | null;
  /** Jobs with an approval in flight. What stops a second dispatch. */
  approving: string[];
};

/** What a command answered. A refusal names itself; it never renders as silence. */
export type Outcome =
  | { ok: true }
  | { ok: false; why: "not_connected" }
  | { ok: false; why: "empty_brief" }
  | { ok: false; why: "already_approving" }
  | { ok: false; why: "refused"; error: WireError }
  | { ok: false; why: "transport"; detail: string };

/** What the create form collects, before it becomes a `ProposeJob`. */
export type Draft = {
  workflowId: string;
  manifestId: string;
  origin: string;
  urgency: string;
  model: string;
  atomic: boolean;
  /** Free text. Refused empty before the Job is created. */
  brief: string;
};

/** The whole preload surface, and therefore everything the renderer can reach. */
export type BridgeApi = {
  protocolVersion: () => number;
  state: () => Promise<BridgeState>;
  subscribe: (onState: (state: BridgeState) => void) => () => void;
  proposeJob: (draft: Draft) => Promise<Outcome>;
  approveDispatch: (jobId: string) => Promise<Outcome>;
};

/** The channels the preload is allowed to name. There is no general `invoke`. */
export const CHANNELS = {
  state: "bridge:state",
  changed: "bridge:changed",
  proposeJob: "bridge:propose-job",
  approveDispatch: "bridge:approve-dispatch",
} as const;
