// What each of Bridge's failures says.
//
// **A builder each, not one message.** Fleet unreachable, a renderer that
// threw, and a Job the store refused are three different situations demanding
// three different things, and folding them into one sentence is the defect this
// file exists to repair. They share a shape — `Failure notice` — the way six
// Job states share one row shape.
//
// **Bridge mints a code for each of its own faults, and they are declared
// here.** Only one of these six failures crosses the wire, so only one arrives
// with a code — and the error treatment's `always` is what separates an error
// from a failed Job, both of which are the same red. The rule that a code's
// declaration lives beside the variant that raises it is kept rather than
// broken: the variant is the builder below, and the declaration sits above it.
//
// The namespace and the argument for it are in `codes.ts` beside `ErrorCode`
// in `@armada/components`. **`cargo xtask verify-error-codes` collects these
// too**, since `#345`: it reads both languages and fails on a duplicate within
// either, so a second declaration of one code names both sites rather than
// resting on somebody having read this file.
//
// **Each failure also says which class it is, beside the code and for the same
// reason.** The class is a claim about Fleet's state, which nothing that draws
// a notice can make — so `FailureNotice` passed `fault` as a literal and every
// one of these drew red, including the two where Fleet is alive and restarting
// it is the wrong move. #344.

import { File } from "lucide-react";
import type {
  BridgeCode,
  DebugField,
  DebugPayload,
  ErrorClass,
  FailureDetail,
  FailureMachineValue,
} from "@armada/components";

import type { BridgeIdentity } from "@armada/protocol";
import type { Connection } from "@armada/protocol";
import { PROTOCOL_VERSION } from "@armada/protocol";
import type { Outcome, UnreadableJob, WireError } from "@armada/protocol";
import { spoken } from "@armada/protocol";
import { elapsed } from "./fleet";
import type { Statement } from "./fleet";
import type { Uncaught } from "./uncaught";

/**
 * The machine record of a failure, minus the instant it is taken.
 *
 * **Built here, beside the sentence, and not derived from the fold.** The two
 * lists look alike and are not the same artifact: `details` carries prose
 * labels a person reads on screen, and this carries wire spellings a person
 * greps a log for. Deriving one from the other would put `Runtime file` into
 * an issue body as a key, and lowercasing a `WireError`'s own `fields` keys
 * would corrupt the only ones that were already right.
 *
 * `at` is added at the moment of copying rather than here — the payload is
 * built on every render and the timestamp is a fact about the press.
 * `copyDebugInfoFor` in `FailureSurface.tsx` is what stamps it, and it is the
 * same function `c` runs.
 *
 * **The app's voice is not in here.** What the old hand-rolled report put on
 * the clipboard was the headline and the `next` sentence — what the screen
 * said, not what the machine had. A reader who was not there needs the second:
 * `message` is the machine's own words and the fields under it say more than a
 * sentence could. The log paths survive as fields, because a report that names
 * a failure without naming the file the rest of it is written in is one
 * somebody has to answer with a question.
 */
export type FailureFacts = Omit<DebugPayload, "at" | "code"> & {
  /**
   * The code, required here where the payload leaves it optional.
   *
   * **One field, read twice.** The chip on screen and the `code` row in the
   * copied artifact are the same value by construction rather than by two
   * call sites agreeing — which is the same reason `debugInfo` is the only
   * thing that formats a payload. `DebugPayload` keeps it optional because a
   * caller outside this file may genuinely have none; every failure built
   * here has one, and the type is where that stops being a convention.
   */
  code: string;
};

export type Failure = {
  /**
   * Which of the two error classes this is. The rule and its argument are the
   * error contract's; what it comes to here is that **degraded asserts the
   * work is still happening and only the reading of it stopped**, so exactly
   * the two connection states where Bridge verified the pid are degraded and
   * everything else — every command, every refused read, every throw inside
   * Bridge — is a fault. Each declaration below carries its own reasoning.
   */
  kind: ErrorClass;
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
  /** What leaves the machine when somebody quotes this failure. */
  payload: FailureFacts;
};

/**
 * What Bridge knows about versions, which is protocol versions and nothing
 * else. **Bridge holds no application version anywhere** — nothing publishes
 * one to the renderer — so the payload says "bridge protocol 5.2" rather than
 * inventing a number a reader would take for a release.
 */
const BRIDGE_PROTOCOL = spoken(PROTOCOL_VERSION);

/**
 * Both protocol versions, on every failure.
 *
 * Fleet's used to reach only the one builder that is handed a `Connection`, so
 * four of the five payloads ended on a half tail — wrong for a refusal above
 * all, since a command Fleet refused came from a Fleet whose version Bridge
 * read before it connected. It is carried on `BridgeIdentity` now, derived
 * once where the connection is published, and every builder reads it here.
 *
 * Absent stays absent: three connection states never got a version to read, and
 * `debugInfo` omits the row rather than printing a blank one.
 */
function versions(bridge: BridgeIdentity): Pick<DebugPayload, "bridgeProtocol" | "fleetProtocol"> {
  return {
    bridgeProtocol: BRIDGE_PROTOCOL,
    ...(bridge.fleetProtocol === null ? {} : { fleetProtocol: bridge.fleetProtocol }),
  };
}

/**
 * Bridge's own machine log, as a payload field.
 *
 * It is not a wire field and never will be — it is a path on the filesystem of
 * the machine the window is on — but it is the one place the rest of what
 * happened is written down, and a report that names the failure without naming
 * the file is a report somebody has to answer with "where is your log".
 * Absent, not blank, where no path resolves.
 */
function logField(bridge: BridgeIdentity): DebugField[] {
  return bridge.auditPath === null ? [] : [{ key: "bridge_log", value: bridge.auditPath }];
}

/**
 * A thrown exception's frames, as the ordered list a chain is.
 *
 * **A stack is a cause chain.** It is the same artifact a `WireError` carries
 * flattened to strings, ordered innermost first, and putting it here rather
 * than into a `fields` value is what keeps the payload's aligned columns from
 * being blown apart by a forty-line value. The first line of a JS stack repeats
 * the message, which is already its own row, so it is dropped.
 */
function frames(stack: string | null, message: string): string[] {
  if (stack === null) return [];
  const lines = stack
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line !== "");
  const [first] = lines;
  return first !== undefined && first.includes(message) ? lines.slice(1) : lines;
}

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
 * `null` where there is nothing wrong. A state with no `next` has nothing to
 * say and takes no notice; a state that is not a fault takes none either, which
 * is why `connected` is answered below rather than filtered out by the guard.
 *
 * **The four runtime-file answers stay four.** Which one it was is the first
 * row of the fold, because only one of the four — running and silent — is
 * worth waiting on, and the other three need somebody to start Fleet.
 *
 * **This is the builder where both classes appear**, and the line falls exactly
 * where the pid check falls: `unreachable` and `version_skew` are the two where
 * Bridge verified the pid, so Fleet is known alive and only the reading has
 * stopped. The other two draw red because Fleet is either absent or unproven.
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
 * A region of the window threw while drawing and its boundary caught it.
 *
 * One code and not one per region. **A region names what stopped drawing, not
 * what went wrong**, so a code per region would have as many values as the app
 * has boundaries and would say nothing about the fault — which is the whole
 * objection to having drawn the region in the chip. The region travels as a
 * field, where a reader can join it to the component.
 */
const RENDER_BOUNDARY: BridgeCode = "bridge.render.boundary";

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
    // **A fault, not the stale view it resembles.** Fleet is fine and Jobs are
    // progressing, but what failed is Bridge's own act: this region will not
    // draw on the next render either, and no waiting makes it current.
    kind: "fault",
    headline: `Bridge could not draw ${region}`,
    // The exception's own words, not the headline: the headline names the
    // region in the app's voice, and a person reading this in an issue needs
    // what threw. A minted code and no run id — the code names a fault Bridge
    // knows the kind of, and the run id would have named a Fleet process this
    // never reached.
    payload: {
      code: RENDER_BOUNDARY,
      message: caught.message,
      fields: [
        { key: "region", value: region },
        ...(caught.component === null
          ? []
          : [{ key: "component", value: caught.component }]),
        { key: "window_usable", value: String(usable) },
        ...logField(bridge),
      ],
      // The thrown stack where React gave one, and the component stack where
      // it did not: both are ordered lists of what was doing what, which is
      // what a chain is. The thrown stack is preferred because it carries file
      // and line, and the component stack carries neither.
      chain:
        caught.stack === null
          ? frames(caught.where, caught.message)
          : frames(caught.stack, caught.message),
      ...versions(bridge),
    },
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
        // cannot know.
        ...(named ? [{ key: "job_log", value: `.armada/logs/${row.job_id}.jsonl` }] : []),
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
 * **The one failure whose code Bridge did not mint**, and nothing here
 * interprets it. It is opaque to Bridge — looked up, never parsed — and the
 * message is what renders when the lookup misses. It carries no `bridge.`
 * prefix, which is how a reader tells at a glance that Fleet raised it and a
 * manifest holds what it means.
 *
 * The whole `fields` map and the whole `chain` are folded away rather than
 * summarised, because a refusal's `message` names one problem even where
 * several exist.
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
    // **A fault, read off the situation and not off the code.** A command was
    // refused, so Armada did not do the thing — and the code here is opaque,
    // so deriving a class from it would read a value the contract forbids.
    kind: "fault",
    // The only one of the six with everything the contract guarantees. The
    // wire's `fields` keys pass through with their own spelling: they are what
    // somebody greps a log for, and rewriting them into prose would break the
    // one join this payload exists to make.
    payload: {
      code: error.code,
      message: error.message,
      run_id: error.run_id,
      ...(error.job_id === undefined ? {} : { job_id: error.job_id }),
      ...(error.drone_id === undefined ? {} : { drone_id: error.drone_id }),
      ...(error.step_id === undefined ? {} : { step_id: error.step_id }),
      fields: [
        ...Object.entries(error.fields ?? {}).map(([key, value]) => ({
          key,
          value: String(value),
        })),
        ...logField(bridge),
      ],
      chain: error.chain ?? [],
      ...versions(bridge),
    },
    headline: error.message,
    next: "Nothing was sent. Change what the command names, or read the log.",
    detailsLabel: "What Fleet refused",
    details,
    values: [...machineLog(bridge), ...fleetRun(error.run_id)],
    note: "The run names Fleet's process for this session, not this one failure. It is what joins this to Fleet's log lines.",
  };
}

/** A promise rejected with nothing waiting on it. */
const UNCAUGHT_REJECTION: BridgeCode = "bridge.uncaught.rejection";

/** Something threw outside a render, where no boundary could catch it. */
const UNCAUGHT_THROW: BridgeCode = "bridge.uncaught.throw";

/**
 * A throw or a rejection no boundary saw.
 *
 * The two are told apart rather than folded together: a rejection is a command
 * that never answered, and a throw is a handler that stopped halfway. What a
 * person does about them is not the same.
 *
 * **Two codes for the same reason the sentence is two sentences.** `from` is
 * already a field, and a reader who has only the chip would otherwise have to
 * open the payload to learn the one thing that decides what to do.
 */
export function uncaughtFailure(uncaught: Uncaught, bridge: BridgeIdentity): Failure {
  const details: FailureDetail[] = [{ label: "Message", value: uncaught.message }];
  if (uncaught.stack !== null) details.push({ label: "Stack", value: uncaught.stack.trim() });

  return {
    // **A fault, both ways.** Something inside Bridge stopped halfway. Fleet's
    // state is not what this is about and may be perfectly healthy — the
    // process that failed is this one.
    kind: "fault",
    // `from` leads the fields because it is the difference that matters: a
    // rejection is a command that never answered, and a throw is a handler
    // that stopped halfway. It is the same difference the code carries, and
    // both are here rather than one: the chip is quoted from a screenshot and
    // the field is grepped out of a log.
    payload: {
      code: uncaught.from === "rejection" ? UNCAUGHT_REJECTION : UNCAUGHT_THROW,
      message: uncaught.message,
      fields: [{ key: "from", value: uncaught.from }, ...logField(bridge)],
      chain: frames(uncaught.stack, uncaught.message),
      ...versions(bridge),
    },
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

/** Bridge sent a command and no answer came back inside the wait. */
const COMMAND_TIMED_OUT: BridgeCode = "bridge.command.timed_out";

/** The request itself failed, so whether Fleet read it is unknown. */
const COMMAND_UNREACHABLE: BridgeCode = "bridge.command.unreachable";

/** Fleet answered a status, and the body under it was not a refusal. */
const COMMAND_UNANSWERABLE: BridgeCode = "bridge.command.unanswerable";

/**
 * A command Fleet did not answer.
 *
 * **The sixth failure, and the one that used to be a line of text.** A refusal
 * carries a `WireError` and reaches `refusalFailure`; everything else on the
 * same seam — a wait that ran out, a socket that failed, a status with no
 * refusal under it — was drawn as a single `Alert` reading "Fleet did not
 * answer: <the machine's words>". That sentence has no code, no fold, and
 * nothing to copy, so a person who hit it had one line and no way to hand it
 * on. It is the generic message this file exists to repair, and it survived
 * here because a transport failure is the one seam failure with no envelope.
 *
 * **Bridge mints the code, and the `bridge.` prefix is what keeps that honest**
 * — the fault may well be Fleet's, and what Bridge is naming is the condition
 * it observed. Three of them, because the three take three different next
 * steps, which `TransportFault` states.
 *
 * **No run id, on any of the three.** Two never got an answer to carry one, and
 * the third got a body Bridge could not read one out of. A labelled blank would
 * claim Bridge looked and Fleet sent none.
 */
export function transportFailure(
  outcome: Extract<Outcome, { ok: false; why: "transport" }>,
  bridge: BridgeIdentity,
): Failure {
  const fault = outcome.fault;
  const asked = `${fault.method} ${fault.path}`;
  const base = {
    // **All three are faults**, whatever Fleet's state turns out to be: what
    // failed is a command rather than a view. The third of them proves Fleet
    // alive and answering and is still one, because the act did not complete
    // and no wait completes it. The degraded thing on this seam is the
    // *connection*, which `fleetFailure` draws and `unreachable` points at.
    kind: "fault" as const,
    detailsLabel: "What Bridge asked",
    values: machineLog(bridge),
  };
  const fields: DebugField[] = [
    { key: "why", value: fault.why },
    { key: "method", value: fault.method },
    { key: "path", value: fault.path },
    { key: "detail", value: outcome.detail },
  ];
  const details: FailureDetail[] = [
    { label: "Route", value: asked },
    { label: "Detail", value: outcome.detail },
  ];

  switch (fault.why) {
    case "timed_out": {
      const waited = elapsed(fault.waitedMs);
      return {
        ...base,
        headline: `Fleet did not answer ${asked} inside ${waited}`,
        // **Never "nothing was sent".** The request reached Fleet and the wait
        // ran out on this side, so the act may have been carried out — and a
        // sentence telling somebody to send it again would be how a request
        // becomes two Jobs. The board is the thing that knows.
        next: "The command may still have been carried out. Read the board before sending it again.",
        payload: {
          code: COMMAND_TIMED_OUT,
          message: `Fleet did not answer ${asked} inside ${waited}`,
          fields: [...fields, { key: "waited_ms", value: String(fault.waitedMs) }, ...logField(bridge)],
          ...versions(bridge),
        },
        details: [...details, { label: "Waited", value: waited }],
        note: "The wait is Bridge's, not Fleet's. Fleet has its own budget on a call it makes, and a refusal from it would have arrived with a code — this is Bridge giving up first, so there is no Fleet run id to quote.",
      };
    }

    case "unreachable":
      return {
        ...base,
        headline: `Bridge could not reach Fleet for ${asked}`,
        // Deliberately not "nothing was sent": a socket that failed may have
        // failed after the bytes went out, and deciding otherwise on no
        // evidence is what makes a duplicate.
        next: "Whether Fleet read it is unknown. Read the board, then send it again.",
        payload: {
          code: COMMAND_UNREACHABLE,
          message: outcome.detail,
          fields: [...fields, ...logField(bridge)],
          ...versions(bridge),
        },
        details,
        note: "The connection failed rather than the command. If the status bar also says Fleet is unreachable, that notice is the one worth sending.",
      };

    case "unanswerable":
      return {
        ...base,
        headline: `Fleet answered ${fault.status} on ${asked}, and Bridge could not read it`,
        // Fleet is up. Retrying sends the same request down the same route to
        // the same disagreement, so the sentence points at the log instead.
        next: "Fleet is running and answered. Nothing here will change by sending it again — read the log.",
        payload: {
          code: COMMAND_UNANSWERABLE,
          message: `Fleet answered ${fault.status} with a body that is not a refusal`,
          fields: [...fields, { key: "status", value: String(fault.status) }, ...logField(bridge)],
          ...versions(bridge),
        },
        details: [...details, { label: "Status", value: String(fault.status) }],
        // The two protocol versions are already on the payload's tail, and
        // this is the one failure where they are the first thing to look at.
        note: "A status with no refusal under it is the two sides disagreeing about the route, not a job going wrong. The protocol versions on this record are what to check first.",
      };
  }
}
