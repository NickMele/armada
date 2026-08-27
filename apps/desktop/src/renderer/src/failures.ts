// What each of Bridge's three failures says.
//
// **Three builders, not one message.** Fleet unreachable, a renderer that
// threw, and a Job the store refused are three different situations demanding
// three different things, and folding them into one sentence is the defect this
// file exists to repair. They share a shape — `Failure notice` — the way six
// Job states share one row shape.
//
// Nothing here mints an error code. A code's declaration lives beside the
// variant that raises it, and a code invented in Bridge would be in no manifest
// and mean nothing to the lookup Bridge does.

import { File } from "lucide-react";
import type { FailureDetail, FailureMachineValue } from "@armada/components";

import type { BridgeIdentity, Connection } from "../../shared/bridge";
import type { UnreadableJob, WireError } from "../../shared/protocol";
import { spoken } from "../../shared/version";
import { elapsed } from "./fleet";
import type { Statement } from "./fleet";
import type { Uncaught } from "./uncaught";

export type Failure = {
  /** What broke, one sentence, in the app's voice. */
  headline: string;
  /** What to do. Never absent. */
  next: string;
  /** How the fold is named, so the reader knows what is under it before opening. */
  detailsLabel: string;
  details: FailureDetail[];
  values: FailureMachineValue[];
  /** What the machine values do not say. */
  note: string;
};

/**
 * Where Bridge's own line goes. Named, not written — nothing appends to this
 * file yet, which is reported rather than papered over with a friendlier
 * sentence.
 */
function machineLog(bridge: BridgeIdentity): FailureMachineValue[] {
  if (bridge.auditPath === null) return [];
  return [{ icon: File, iconLabel: "Log", value: bridge.auditPath, copyValue: bridge.auditPath }];
}

/**
 * Fleet's run id, and only Fleet's.
 *
 * **It is labelled "Fleet run" because that is what it is.** Measured against a
 * live daemon: the id is minted once per Fleet process and every answer clones
 * it, so four different refusals in one session quote one value. Calling it an
 * error id or a reference would promise that it points at this failure, and the
 * first person to quote it would find it names a whole session.
 *
 * Nothing else on this surface carries one. Bridge mints none — an id minted by
 * a process that writes no log line joins to nothing — and a renderer crash
 * never went near Fleet, so it has none to show and shows none.
 */
function fleetRun(runId: string): FailureMachineValue[] {
  if (runId === "") return [];
  return [{ value: runId, copyValue: runId, meta: "Fleet run" }];
}

/**
 * Fleet, when the one connection is not a connection.
 *
 * `null` where there is nothing wrong. A state with no `next` has nothing to
 * say and takes no notice; a state that is not a fault takes none either, which
 * is why `connected` is answered below rather than filtered out by the guard.
 *
 * **The four runtime-file answers stay four.** Which one it was is the first
 * row of the fold, because only one of the four — running and silent — is
 * worth waiting on, and the other three need somebody to start Fleet.
 */
export function fleetFailure(
  connection: Connection,
  statement: Statement,
  bridge: BridgeIdentity,
  now: number,
): Failure | null {
  if (statement.next === null) return null;

  const base = {
    headline: statement.headline,
    next: statement.next,
    // Bridge never reached Fleet, so there is no Fleet run to quote. What
    // identifies these is in the fold: the runtime file, the pid and the port.
    values: machineLog(bridge),
  };

  switch (connection.state) {
    case "not_running": {
      const absence = connection.absence;
      const details: FailureDetail[] = [
        { label: "Answer", value: absence.why },
        { label: "Runtime file", value: absence.path },
      ];
      if (absence.why === "pid_dead" || absence.why === "pid_held_by_another") {
        details.push({ label: "Pid", value: String(absence.pid) });
      }
      if (absence.why === "pid_held_by_another") {
        details.push({ label: "File wrote", value: absence.wrote });
        details.push({ label: "Holder started", value: absence.holder });
      }
      return {
        ...base,
        detailsLabel: "What the runtime file answered",
        details,
        note:
          absence.why === "pid_held_by_another"
            ? "Bridge did not open a socket. The port in this file is not Fleet's."
            : "Bridge rereads the file every 2 seconds and connects when Fleet writes one.",
      };
    }

    case "runtime_file_refused":
      return {
        ...base,
        detailsLabel: "What the read answered",
        details: [
          { label: "Answer", value: connection.fault.why },
          { label: "Runtime file", value: connection.fault.path },
          { label: "Detail", value: connection.fault.detail },
          ...(connection.fault.why === "probe_failed"
            ? [{ label: "Pid", value: String(connection.fault.pid) }]
            : []),
        ],
        // Not folded into "not running": the read failed, and calling that a
        // Fleet that is down decides on no evidence.
        note: "The read failed, so whether Fleet is running is unknown. Bridge will not connect to a port this file names.",
      };

    case "unreachable":
      return {
        ...base,
        detailsLabel: "What the connection answered",
        details: [
          { label: "Pid", value: String(connection.fleet.pid) },
          { label: "Port", value: String(connection.fleet.port) },
          { label: "Silent for", value: elapsed(now - connection.sinceMs) },
          { label: "Detail", value: connection.detail },
        ],
        note: "Bridge is retrying every 2 seconds. Jobs keep progressing either way.",
      };

    case "version_skew":
      return {
        ...base,
        detailsLabel: "What each side speaks",
        details: [
          { label: "Fleet", value: spoken(connection.speaks) },
          { label: "Bridge", value: spoken(connection.expected) },
          { label: "Pid", value: String(connection.fleet.pid) },
          { label: "Port", value: String(connection.fleet.port) },
        ],
        // Two refusals, and naming which one is the whole use of this fold: a
        // major gap is two binaries from different commits, and a Fleet behind
        // by a minor is the right binaries with the daemon left running.
        note:
          connection.why === "incompatible"
            ? "Bridge did not open a socket. A message from a Fleet on another protocol is not one Bridge can read."
            : "Bridge did not open a socket. This Fleet speaks the same protocol without the additions Bridge now reads, and a field arriving absent mid-Job is worse than not connecting.",
      };

    // None of the three is a fault, so none takes a notice. **`connected` is
    // the one that can carry a `next` anyway** — a Fleet ahead by a minor puts
    // its banner in the status bar, and drawing it here as well would say
    // something is broken when the connection is working.
    //
    // Listed rather than defaulted, so a new connection state is a compile
    // error instead of a silent fall-through to a generic message.
    case "reading":
    case "connecting":
    case "connected":
      return null;
  }
}

/** What the boundary caught, flattened before anything renders it. */
export type Caught = {
  message: string;
  /** The first frame of the component stack, where React gave one. */
  component: string | null;
  /** The component stack, as React wrote it. */
  where: string | null;
  stack: string | null;
};

/**
 * The renderer threw.
 *
 * The headline names the region in the app's voice rather than the class of
 * the exception — what broke, not what threw — and the component the stack
 * names is folded away with the message and the stack itself.
 */
export function rendererFailure(
  caught: Caught,
  region: string,
  bridge: BridgeIdentity,
  usable: boolean,
): Failure {
  const details: FailureDetail[] = [
    { label: "Component", value: caught.component ?? "not named by the stack" },
    { label: "Message", value: caught.message },
  ];
  if (caught.where !== null) details.push({ label: "Where", value: caught.where.trim() });
  if (caught.stack !== null) details.push({ label: "Stack", value: caught.stack.trim() });

  return {
    headline: `Bridge could not draw ${region}`,
    // Safe to state flatly: Bridge and Fleet have independent lifetimes, so a
    // reload reconnects to the running daemon rather than restarting anything.
    next: "Reload Bridge. Fleet keeps running and jobs keep progressing.",
    detailsLabel: "What threw",
    details,
    values: machineLog(bridge),
    // No run id, and no labelled blank where one would go: this never reached
    // Fleet. The component above and the log below are what identify it.
    note: usable
      ? "The rest of the window is still usable. Only this region stopped drawing. This never reached Fleet, so there is no run id: the component and the log identify it."
      : "The whole window stopped drawing, so nothing below it is current. This never reached Fleet, so there is no run id: the component and the log identify it.",
  };
}

/**
 * A Job the store refused.
 *
 * `LoadAllError` returns what loaded beside what failed, so this is one bad row
 * and not a broken board. The two things the wire does not carry are said out
 * loud rather than guessed at: which repository the Job's log is in, and the
 * `run_id` of the read that refused the row.
 */
export function jobFailure(row: UnreadableJob, bridge: BridgeIdentity): Failure {
  const named = row.job_id !== undefined;
  return {
    headline: named ? `Job ${row.job_id} did not load` : "A job did not load",
    next: named
      ? "Every other job on the board is unaffected. Read the fault, or read the job's log."
      : "Every other job on the board is unaffected. The row carries no job id, so there is no log to open.",
    detailsLabel: "What the store refused",
    details: [
      ...(named ? [{ label: "Job", value: row.job_id as string }] : []),
      { label: "Fault", value: row.fault },
    ],
    values: [
      ...(named
        ? [
            {
              icon: File,
              iconLabel: "Log",
              value: `.armada/logs/${row.job_id}.jsonl`,
              copyValue: `.armada/logs/${row.job_id}.jsonl`,
            },
          ]
        : []),
      // Two logs, and the rule between them says they are two: the Job's own,
      // in a repository Bridge cannot name, and Bridge's own machine log.
      ...machineLog(bridge).map((value) => ({ ...value, separated: named })),
    ],
    note: named
      ? "The log path is relative to the job's repository. Fleet does not send which one, and it does not send a run id for the read that refused this row."
      : "Fleet does not send a run id for the read that refused this row.",
  };
}

/**
 * A command Fleet refused.
 *
 * **The only failure here that carries a run id**, because it is the only one
 * minted on the other side of the connection. It names Fleet's run, and the row
 * says so.
 *
 * Nothing here interprets the code. It is opaque to Bridge — looked up, never
 * parsed — and the message is what renders when the lookup misses. The whole
 * `fields` map and the whole `chain` are folded away rather than summarised,
 * because a refusal's `message` names one problem even where several exist.
 */
export function refusalFailure(error: WireError, bridge: BridgeIdentity): Failure {
  const details: FailureDetail[] = [
    { label: "Code", value: error.code },
    { label: "Message", value: error.message },
  ];
  if (error.job_id !== undefined) details.push({ label: "Job", value: error.job_id });
  if (error.drone_id !== undefined) details.push({ label: "Drone", value: error.drone_id });
  if (error.step_id !== undefined) details.push({ label: "Step", value: error.step_id });
  for (const [key, value] of Object.entries(error.fields ?? {})) {
    details.push({ label: key, value: String(value) });
  }
  if ((error.chain ?? []).length > 0) {
    details.push({ label: "Chain", value: error.chain.join("\n") });
  }

  return {
    headline: error.message,
    next: "Nothing was sent. Change what the command names, or read the log.",
    detailsLabel: "What Fleet refused",
    details,
    values: [...machineLog(bridge), ...fleetRun(error.run_id)],
    note: "The run names Fleet's process for this session, not this one failure. It is what joins this to Fleet's log lines.",
  };
}

/**
 * A throw or a rejection no boundary saw.
 *
 * The two are told apart rather than folded together: a rejection is a command
 * that never answered, and a throw is a handler that stopped halfway. What a
 * person does about them is not the same.
 */
export function uncaughtFailure(uncaught: Uncaught, bridge: BridgeIdentity): Failure {
  const details: FailureDetail[] = [{ label: "Message", value: uncaught.message }];
  if (uncaught.stack !== null) details.push({ label: "Stack", value: uncaught.stack.trim() });

  return {
    headline:
      uncaught.from === "rejection"
        ? "Something Bridge asked for never answered"
        : "Something Bridge was doing stopped halfway",
    next: "Nothing on the board changed. Try it again, and reload Bridge if it repeats.",
    detailsLabel: "What was thrown",
    details,
    values: machineLog(bridge),
    note: "No error boundary sees this: a boundary catches a render, and this happened outside one. There is no run id, because this may never have reached Fleet at all.",
  };
}

/**
 * The whole failure as one block of text.
 *
 * **This is what the report action carries.** A person reporting a failure
 * should not be retyping a stack, a path or a run id off a screen, and a report
 * that arrives without the run id joins to no line in any log.
 */
export function reportOf(failure: Failure, at: string): string {
  const lines = [
    failure.headline,
    failure.next,
    "",
    ...failure.details.map((detail) => `${detail.label}: ${detail.value}`),
    ...failure.values.map((value) =>
      value.meta === undefined ? value.value : `${String(value.meta)}: ${value.value}`,
    ),
    `Reported at: ${at}`,
  ];
  return lines.join("\n");
}
