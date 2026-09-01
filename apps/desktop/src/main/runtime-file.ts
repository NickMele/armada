// How Bridge finds Fleet, and how it knows the Fleet it found is the one the
// file names.
//
// `crates/fleet/src/runtime.rs` is the authority on this file and this module
// is the other half of that contract: same path, same four answers, same probe.
// The one thing not read from the file is the host — `127.0.0.1` is a constant
// on Fleet's side and a constant here, so there is no field an edited file
// could put a routable address into.
//
// **Unknown fields are ignored on purpose.** This is a contract between two
// independently versioned binaries, and a reader that refuses a field it has
// not heard of turns every additive change into a breaking one. A field that is
// missing or the wrong type is a different thing and is refused.

import { readFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { join } from "node:path";

import type { BridgeIdentity } from "@armada/protocol";
import type { Absence, FleetIdentity, RuntimeFault } from "@armada/protocol";
import { versionOf } from "@armada/protocol";

/** Not `fleet.pid`: it carries four fields, and something would eventually `cat` it. */
const FILE_NAME = "fleet.json";

/** The machine-level sink, named by `docs/concepts/log-envelope.md`. */
const AUDIT_NAME = "audit.jsonl";

/**
 * Loopback, always. Fleet answers commands that spawn processes against a real
 * repository, so the host is structural rather than configured — it is not in
 * the file and cannot be read from one.
 */
export const HOST = "127.0.0.1";

/** What is at the runtime file's path. */
export type Presence =
  | { at: "absent"; absence: Absence }
  | { at: "stale"; absence: Absence }
  | { at: "running"; fleet: FleetIdentity; path: string }
  | { at: "refused"; fault: RuntimeFault };

/**
 * Where machine-level state lives. Application Support, because Armada is a
 * desktop application rather than a command-line tool.
 */
export function machinePath(home: string | undefined): string | null {
  const dir = machineDir(home);
  return dir === null ? null : join(dir, FILE_NAME);
}

/**
 * Where a Bridge failure says its line is. **Machine-level, not per-Job**: a
 * connection that never reached Fleet has no `job_id` to file itself under.
 */
export function auditPath(home: string | undefined): string | null {
  const dir = machineDir(home);
  return dir === null ? null : join(dir, AUDIT_NAME);
}

/**
 * The identity a window starts from: its own log path, and no Fleet.
 *
 * Fleet's protocol version is the other half of what every failure's payload
 * carries, and there is none until a runtime file has been read and believed —
 * which is what this module does next. `identifying` keeps it current from
 * there on.
 */
export function startingIdentity(home: string | undefined): BridgeIdentity {
  return { auditPath: auditPath(home), fleetProtocol: null };
}

function machineDir(home: string | undefined): string | null {
  if (home === undefined || home === "") return null;
  return join(home, "Library", "Application Support", "Armada");
}

/** What a process reported as its start time. Compared for equality, never parsed. */
type Holder = { held: false } | { held: true; startedAt: string } | { held: null; detail: string };

/** The largest pid the platform can express. Above it names no process. */
const PID_CEILING = 2147483647;

/**
 * Who holds `pid`, if anybody — the same `ps -o lstart=` spelling Fleet uses,
 * which is why that spelling was chosen: the runtime file is a contract between
 * a Rust process and a Node one, and an identity check only one of them can
 * perform is not one.
 *
 * **Pid zero is never held.** It names the caller's own process group to
 * `kill(2)` and nothing at all here, and zero is what a half-written file reads
 * as.
 */
export function holderOf(pid: number): Holder {
  if (!Number.isInteger(pid) || pid <= 0 || pid > PID_CEILING) return { held: false };

  const run = spawnSync("ps", ["-o", "lstart=", "-p", String(pid)], { encoding: "utf8" });
  if (run.error !== undefined) {
    return { held: null, detail: run.error.message };
  }
  // Told apart by stderr rather than by the exit code: an absent pid and a
  // refused argument both exit non-zero, and only one means nothing is there.
  const complaint = (run.stderr ?? "").trim();
  if (complaint !== "") return { held: null, detail: complaint };
  const reading = (run.stdout ?? "").trim();
  if (reading === "") return { held: false };
  return { held: true, startedAt: reading };
}

/** The four fields, checked. Anything else in the object is ignored. */
function identity(parsed: unknown): FleetIdentity | null {
  if (typeof parsed !== "object" || parsed === null) return null;
  const record = parsed as Record<string, unknown>;
  const { protocol_version: version, pid, port, started_at: startedAt } = record;
  // A pair, or the bare integer a Fleet from before the pair wrote. Refusing
  // the old form would tell a person their runtime file is corrupt when it is
  // only old — and skew is the reading that belongs on that screen.
  const speaks = versionOf(version);
  if (speaks === null || typeof pid !== "number") return null;
  if (typeof port !== "number" || typeof startedAt !== "string") return null;
  return { protocolVersion: speaks, pid, port, startedAt };
}

/**
 * Read the runtime file and decide what it describes.
 *
 * The probe is part of reading, so there is no shape here through which a pid
 * reaches a connection attempt unchecked.
 */
export async function read(path: string): Promise<Presence> {
  let text: string;
  try {
    text = await readFile(path, "utf8");
  } catch (cause) {
    const code = (cause as NodeJS.ErrnoException).code;
    if (code === "ENOENT") return { at: "absent", absence: { why: "no_runtime_file", path } };
    return { at: "refused", fault: { why: "unreadable", path, detail: detailOf(cause) } };
  }

  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch (cause) {
    return { at: "refused", fault: { why: "undecodable", path, detail: detailOf(cause) } };
  }

  // A file that will not parse is refused, not ignored. Fleet writes to a
  // sibling and renames, so a reader sees a whole version or the previous one —
  // never half of either. Something that is not Fleet wrote this.
  const fleet = identity(parsed);
  if (fleet === null) {
    return {
      at: "refused",
      fault: { why: "undecodable", path, detail: "the four fields Fleet writes are not all there" },
    };
  }

  const holder = holderOf(fleet.pid);
  if (holder.held === null) {
    return {
      at: "refused",
      fault: { why: "probe_failed", path, pid: fleet.pid, detail: holder.detail },
    };
  }
  if (holder.held === false) {
    return { at: "stale", absence: { why: "pid_dead", path, pid: fleet.pid } };
  }
  if (holder.startedAt !== fleet.startedAt) {
    // The row a bare liveness check gets wrong. Something holds the pid Fleet
    // used to hold, and connecting to the port anyway is the failure the
    // `started_at` field exists to prevent.
    return {
      at: "stale",
      absence: {
        why: "pid_held_by_another",
        path,
        pid: fleet.pid,
        wrote: fleet.startedAt,
        holder: holder.startedAt,
      },
    };
  }
  return { at: "running", fleet, path };
}

function detailOf(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause);
}
