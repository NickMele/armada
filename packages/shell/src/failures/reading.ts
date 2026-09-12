// What Bridge could not read: Fleet, and one row Fleet sent.
//
// **Neither was provoked by a press.** Both are read off published state on
// every render, which is what puts them together and what separates them from
// the answer to a command. They are one claim at two scales — a connection that
// failed means no row on the board is current, and an unreadable row means one
// is not — and "one bad row is not a broken board" only says something where
// both are drawn from the same place.
//
// **Five codes, and both error classes appear here and nowhere else.** The line
// falls exactly where the pid check falls; each declaration argues its own.

import { File } from "lucide-react";
import type {
  BridgeCode,
  DebugField,
  FailureDetail,
} from "@armada/components";

import type { BridgeIdentity } from "@armada/protocol";
import type { Connection } from "@armada/protocol";
import type { UnreadableJob } from "@armada/protocol";
import { spoken } from "@armada/protocol";
import { elapsed } from "../fleet";
import type { Statement } from "../fleet";
import type { Failure, FailureFacts } from "./notice";
import { logField, machineLog, versions } from "./notice";

/**
 * Fleet's runtime file named no process Bridge could connect to. **A fault**:
 * no Fleet is running, so no Job is progressing and nothing becomes current on
 * its own. The fix is the one a degraded notice must never send anybody to.
 */
const FLEET_NOT_RUNNING: BridgeCode = "bridge.fleet.not_running";

/**
 * Fleet's runtime file could not be read, so its liveness is unknown.
 *
 * **A fault, and the one the rule has to work on.** It sits beside two degraded
 * states and looks like them. Degraded asserts Fleet is alive, and here the
 * read that would have established it failed — an unknown is not a claim, and
 * drawing it as a stale view says wait for a daemon that may not exist.
 */
const FLEET_RUNTIME_FILE_REFUSED: BridgeCode = "bridge.fleet.runtime_file_refused";

/**
 * Fleet's process is alive and its socket has stopped answering.
 *
 * **Degraded**, and the case the design system names by hand. The pid was
 * verified, so Fleet is running and Jobs keep progressing; what stopped is
 * Bridge's reading of them. This builder's own note has said so all along —
 * "Jobs keep progressing either way" — while the notice above it drew red.
 */
const FLEET_UNREACHABLE: BridgeCode = "bridge.fleet.unreachable";

/**
 * Fleet speaks a protocol Bridge will not open a socket for.
 *
 * **Degraded.** The pid was verified and the socket answered with a version, so
 * Fleet is alive and dispatching; Bridge declined to read it rather than failed
 * to. Independent lifetimes are the point, so a person has a stale board rather
 * than a stopped fleet.
 *
 * **Standing rather than self-clearing, which does not make it a fault.** The
 * class is about whether the work is still happening, not about recovery.
 */
const FLEET_VERSION_SKEW: BridgeCode = "bridge.fleet.version_skew";

/**
 * Fleet, when the one connection is not a connection.
 *
 * `null` where nothing is wrong — a state with no `next` has nothing to say,
 * and a state that is not a fault takes no notice, which is why `connected`
 * is answered below rather than filtered out.
 *
 * **The four runtime-file answers stay four** — which one it was is the
 * fold's first row, since only running-and-silent is worth waiting on; the
 * other three need somebody to start Fleet.
 *
 * **Both classes appear here, exactly where the pid check falls**:
 * `unreachable`/`version_skew` verified the pid — Fleet known alive, only
 * the reading stopped. The other two draw red: Fleet absent or unproven.
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

  /**
   * The connection state is the machine's own name for what happened, and it
   * leads the fields for that reason: `not_running` and `unreachable` are two
   * different things to do about it, and the sentence above renders both as
   * Fleet being unavailable.
   *
   * **A minted code and no run id**, which is not an inconsistency: a code
   * names a kind of failure and Bridge knows which of the four this is, while
   * a run id names a process instance and the one that would be quoted here is
   * Fleet's, which Bridge never reached. The four codes are four because the
   * four answers are four — `not_running` and `unreachable` are opposite fixes,
   * and one code over both would be the generic message this file exists to
   * repair, moved into the field a person quotes.
   */
  function facts(code: BridgeCode, fields: DebugField[]): FailureFacts {
    return {
      code,
      message: statement.headline,
      fields: [
        { key: "connection", value: connection.state },
        ...fields,
        ...logField(bridge),
      ],
      ...versions(bridge),
    };
  }

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
      const absent: DebugField[] = [
        { key: "why", value: absence.why },
        { key: "runtime_file", value: absence.path },
      ];
      if (absence.why === "pid_dead" || absence.why === "pid_held_by_another") {
        absent.push({ key: "pid", value: String(absence.pid) });
      }
      if (absence.why === "pid_held_by_another") {
        absent.push({ key: "file_wrote", value: absence.wrote });
        absent.push({ key: "holder_started", value: absence.holder });
      }
      return {
        ...base,
        kind: "fault",
        payload: facts(FLEET_NOT_RUNNING, absent),
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
        kind: "fault",
        payload: facts(FLEET_RUNTIME_FILE_REFUSED, [
          { key: "why", value: connection.fault.why },
          { key: "runtime_file", value: connection.fault.path },
          { key: "detail", value: connection.fault.detail },
          ...(connection.fault.why === "probe_failed"
            ? [{ key: "pid", value: String(connection.fault.pid) }]
            : []),
        ]),
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
        kind: "degraded",
        payload: facts(FLEET_UNREACHABLE, [
          { key: "pid", value: String(connection.fleet.pid) },
          { key: "port", value: String(connection.fleet.port) },
          { key: "silent_for", value: elapsed(now - connection.sinceMs) },
          { key: "detail", value: connection.detail },
        ]),
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
        kind: "degraded",
        payload: facts(FLEET_VERSION_SKEW, [
          { key: "why", value: connection.why },
          // Not the tail's `fleet protocol`, and not always equal to it: the
          // tail carries what the runtime file said, and this is what the
          // socket said. A Fleet restarted under a live connection is not the
          // one the file described, and the two disagreeing is the whole
          // finding.
          { key: "fleet_speaks", value: spoken(connection.speaks) },
          { key: "pid", value: String(connection.fleet.pid) },
          { key: "port", value: String(connection.fleet.port) },
        ]),
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

/**
 * Fleet returned a job row it could not read, with no code of its own.
 *
 * **The one minted code that names something Bridge did not do.** The failure
 * is the store's; what Bridge mints a code for is the condition it observed —
 * a row arriving unreadable — and the `bridge.` prefix is what keeps that
 * honest, because it says which process minted the value rather than which one
 * broke. Fleet is still the sole authority for the ids on the row, and nothing
 * here invents one of those.
 *
 * Minting here rather than leaving the row codeless is the whole point of the
 * decision: an exception in the rule that separates an error from a status is
 * an exception on the surface a person meets it on.
 */
const JOB_UNREADABLE: BridgeCode = "bridge.job.unreadable";

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
    // **A fault.** Fleet answered: this is a row Fleet refused, not a read
    // Bridge missed, and it will be refused again next time. The board around
    // it is current, which is the opposite of a stale view.
    kind: "fault",
    headline: named ? `Job ${row.job_id} did not load` : "A job did not load",
    // **The one wire failure that is not a `WireError`.** `UnreadableJob`
    // carries a job id and a sentence, and no code, no run id and no chain, so
    // the payload of a refused row is thinner than the payload of a refused
    // command by everything the store could have said and did not. The code is
    // Bridge's, minted for the condition; the run id and the chain stay absent,
    // because those name things only Fleet could have supplied.
    payload: {
      code: JOB_UNREADABLE,
      message: row.fault,
      ...(named ? { job_id: row.job_id } : {}),
      fields: [
        { key: "source", value: "job_list.unreadable" },
        // Relative to the Job's repository, which Fleet does not send. Named
        // as it is written on screen rather than resolved to something Bridge
        // cannot know — and the directory rather than the file, because a
        // job's log is named by its handle and a row that would not rebuild
        // cannot say what its handle is.
        ...(named ? [{ key: "job_log", value: ".armada/logs/" }] : []),
        ...logField(bridge),
      ],
      ...versions(bridge),
    },
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
              iconLabel: "Logs",
              value: ".armada/logs/",
              copyValue: ".armada/logs/",
            },
          ]
        : []),
      // Two logs, and the rule between them says they are two: the Job's own,
      // in a repository Bridge cannot name, and Bridge's own machine log.
      ...machineLog(bridge).map((value) => ({ ...value, separated: named })),
    ],
    note: named
      ? "The log path is relative to the job's repository. Fleet does not send which one. A job's log is named by its handle, which this row could not be rebuilt far enough to say, so what is named is the directory it is in. Fleet does not send a run id for the read that refused this row either."
      : "Fleet does not send a run id for the read that refused this row.",
  };
}
