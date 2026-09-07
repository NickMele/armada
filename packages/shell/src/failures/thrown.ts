// What threw inside Bridge, whether or not a boundary was there to catch it.
//
// **Neither of these reached Fleet**, and that is what puts them together:
// there is no run id to quote on either, no envelope under either, and a stack
// is most of what identifies them — which is why `frames` is here and nowhere
// else. Both say so on the notice rather than leaving a labelled blank.
//
// **A render that threw and a throw outside one are still two.** A boundary
// catches a render and names the region that stopped drawing while the rest of
// the window stays usable; a handler that stopped halfway left no region to
// name. Three codes, because a rejection and a throw are different things to do
// about, and the chip is what a person quotes from a screenshot.

import type { BridgeCode, FailureDetail } from "@armada/components";

import type { BridgeIdentity } from "@armada/protocol";
import type { Failure } from "./notice";
import { logField, machineLog, versions } from "./notice";
import type { Uncaught } from "../uncaught";

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
