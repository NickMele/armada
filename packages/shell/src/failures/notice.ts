// The shape every failure has, and the tail every builder puts on one.
//
// **Assembled here rather than in each builder** because all three groups need
// the same three things: both protocol versions, Bridge's own log as a payload
// field, and that same log as a machine value on screen. A builder that
// assembled its own would be a fourth place the log path is spelled.
//
// **Each failure also says which class it is, beside its code and for the same
// reason.** The class is a claim about Fleet's state, which nothing that draws
// a notice can make — so `FailureNotice` passed `fault` as a literal and every
// one of these drew red, including the two where Fleet is alive and restarting
// it is the wrong move. #344. Each declaration beside a builder argues its own.

import { File } from "lucide-react";
import type {
  DebugField,
  DebugPayload,
  ErrorClass,
  FailureDetail,
  FailureMachineValue,
} from "@armada/components";

import type { BridgeIdentity } from "@armada/protocol";
import { PROTOCOL_VERSION } from "@armada/protocol";
import { spoken } from "@armada/protocol";

/**
 * The machine record of a failure, minus the instant it is taken.
 *
 * **Built here, not derived from the fold.** `details` is prose a person
 * reads; this is wire spellings greppable in a log — deriving one from the
 * other would corrupt `WireError`'s own `fields`.
 *
 * `at` is added at copy time — the payload rebuilds every render, and
 * `copyDebugInfoFor` stamps the press.
 *
 * **The app's voice is not in here** — the old report clipboarded the
 * headline and `next` sentence, not what the machine had. `message` is the
 * machine's own words; log paths survive as fields so a failure is never
 * named without its file.
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
export function versions(bridge: BridgeIdentity): Pick<DebugPayload, "bridgeProtocol" | "fleetProtocol"> {
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
export function logField(bridge: BridgeIdentity): DebugField[] {
  return bridge.auditPath === null ? [] : [{ key: "bridge_log", value: bridge.auditPath }];
}

/**
 * Where Bridge's own line goes. Named, not written — nothing appends to this
 * file yet, which is reported rather than papered over with a friendlier
 * sentence.
 */
export function machineLog(bridge: BridgeIdentity): FailureMachineValue[] {
  if (bridge.auditPath === null) return [];
  return [{ icon: File, iconLabel: "Log", value: bridge.auditPath, copyValue: bridge.auditPath }];
}
