// What came of asking main to open a Job's pull request.
//
// **Not the wire, and it is here for `artifacts.ts`'s reason.** Fleet is not
// involved in following a link: no protocol version moves, no Rust DTO has a
// counterpart, and the whole exchange is a renderer asking the process beside
// it to hand an address to the OS. What puts it in this package is that both
// ends of that exchange have to name the same type, and they are two TypeScript
// projects that deliberately cannot see each other — main and preload have Node
// and no DOM, the renderer has the DOM and no Node. This package is the only
// one both reach, which is why `Opened` is in `artifacts.ts` and not in a
// screen.
//
// The address itself is on the wire and always was: `JobDelivery.pull_request`,
// in `protocol.ts`. What was missing was anywhere on screen that could be
// clicked — `#422`.

/**
 * Whether the pull request opened, and why it did not.
 *
 * **Four arms, because a person does four different things about them**, which
 * is the rule `Opened` keeps one seam over. Three of the four are races a
 * person cannot bring about — the link is drawn only where the address is — and
 * each still gets a sentence, because refusing silently is the dead click the
 * whole affordance exists to remove.
 *
 * The sentences are `screens/src/opening.ts`'s. Nothing here writes copy.
 */
export type Followed =
  | { ok: true }
  /** Bridge no longer holds the Job the click came off. Nothing was looked up. */
  | { ok: false; why: "unknown_job" }
  /** Bridge holds the Job and the reading it is holding names no pull request. */
  | { ok: false; why: "no_address" }
  /**
   * The record names something Bridge will not hand to the OS.
   *
   * **Its own arm and never folded into `refused`.** That one is the machine
   * declining an address Bridge was willing to open; this is Bridge declining
   * the address itself, which is a record carrying something no forge wrote.
   * The two send a person to different places — one to their browser, one to
   * what Fleet recorded.
   */
  | { ok: false; why: "not_addressable"; address: string }
  /** The address was handed over and this machine did not open it. */
  | { ok: false; why: "refused"; address: string; detail: string };
