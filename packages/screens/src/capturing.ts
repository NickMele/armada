// Opening the capture window on a server Run, and saying why one did not open
// — #1294, `../../../docs/practices/capture-window.md`.
//
// **Beside `opening.ts` rather than inside it.** Every sentence there is about
// handing an address to the process that owns the shell, which is the design
// system's hard rule; this one is about a window Armada opens itself, which is
// the one exception that rule now names. Two vocabularies would be worse than
// two modules, and a reader who lands on either finds the other named.

import type { CaptureOpened } from "@armada/protocol";

/**
 * Why the capture window did not open, in the app's voice, or `null` because
 * it did.
 *
 * **Every one names what a person can do next**, which is the whole of why
 * there are four rather than one: a Run that stopped, a link the holder does
 * not carry, an address that is not on this machine and a Studio nothing is
 * holding need four different moves.
 */
export function whyNotCaptured(opened: CaptureOpened): string | null {
  if (opened.ok) return null;
  switch (opened.why) {
    case "not_serving":
      return (
        "This run is no longer serving, so there is nothing to open. Start the server again " +
        "from the Studio and the new Run is what capture opens on."
      );
    case "no_address":
      return "This server is no longer on the reading Bridge is holding. Reopen the Studio and try again.";
    case "not_loopback":
      return (
        `This server's link is ${opened.address}, and the capture window only opens a server ` +
        "running on this machine. Open it in the browser instead."
      );
    case "no_studio":
      return (
        "Bridge is not holding a Studio with this Run on it, so a Note captured here would have " +
        "nowhere to land. Open the Studio the Run was started from and try again."
      );
  }
}

/** Asking main to open the capture window — a server id and one of its own links. */
export type OpenCaptureWindow = (serverId: string, url: string) => Promise<CaptureOpened>;

/**
 * Open the capture window on one server link, and answer with the sentence to
 * say where it did not. `null` is success, and is the whole of the
 * confirmation: the window is in front of the person.
 */
export async function captureOn(
  open: OpenCaptureWindow,
  serverId: string,
  url: string,
): Promise<string | null> {
  return whyNotCaptured(await open(serverId, url));
}
