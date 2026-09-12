// What came back from a command, where what came back was a failure.
//
// **Two builders, and they are the two arms of one decision at the call site**:
// a refusal is Fleet declining with an envelope, and a transport failure is a
// command it did not answer at all. Everything else an `Outcome` carries is the
// form saying what it will not send, which is guidance and takes an `Alert`.
//
// **The only failures a person provoked**, which is why each names what
// survives before it names what to do: a command that may or may not have been
// carried out is the one place pressing again makes two Jobs.
//
// Four codes. Three are minted here, and the fourth is Fleet's own — the one
// code on this surface Bridge did not mint, and does not parse.

import type {
  BridgeCode,
  DebugField,
  FailureDetail,
  FailureMachineValue,
} from "@armada/components";

import type { BridgeIdentity } from "@armada/protocol";
import type { Outcome, WireError } from "@armada/protocol";
import { elapsed } from "../fleet";
import type { Failure } from "./notice";
import { logField, machineLog, versions } from "./notice";

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
        next: "The command may still have been carried out. Bridge will read the board again shortly; read it yourself before sending it a second time.",
        payload: {
          code: COMMAND_TIMED_OUT,
          message: `Fleet did not answer ${asked} inside ${waited}`,
          fields: [...fields, { key: "waited_ms", value: String(fault.waitedMs) }, ...logField(bridge)],
          ...versions(bridge),
        },
        details: [...details, { label: "Waited", value: waited }],
        note: "The wait is Bridge's, not Fleet's. A route with a model call inside it answers within a budget of its own and a refusal from it would carry a code — an ordinary command has no such budget, so a slow answer here is not necessarily a stuck Fleet. There is no Fleet run id to quote either way: this is Bridge giving up first.",
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
