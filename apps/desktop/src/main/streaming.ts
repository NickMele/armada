// Forwarding one frame's bytes to the window, a span at a time.

// **The renderer still never reaches Fleet.** This is the whole of what the
// privileged scheme does: read the Job and the frame id out of an address the
// app itself composed, ask the port main already holds, and hand back what came
// — status, the headers a player reads, and the body as a stream. It is
// `frameOf`'s one request with nothing buffered in the middle.

// **Nothing from Electron is imported here.** Main's tests run in node with no
// window, so the fetch arrives as an argument and `index.ts` — the one file
// that holds the shell — passes Chromium's own.

import { askedFor, FRAME_SCHEME } from "../shared/streaming";
import { HOST } from "./runtime-file";

export { FRAME_SCHEME };

/** The port Fleet is on, or `null` where nothing is connected. */
export type PortOf = () => number | null;

/** One request out, as much of `fetch` as this needs. */
export type Fetching = (
  url: string,
  init: { method: "GET"; headers?: Record<string, string> },
) => Promise<Response>;

/**
 * The headers carried back to the window.
 *
 * **A list rather than everything.** What a player needs is what it is, how
 * long it is, which span this is and whether it may ask for another; the rest
 * belongs to the hop and saying it again would be this process claiming
 * something about a connection it did not make. `nosniff` rides along because
 * Fleet meant it about these bytes.
 */
const CARRIED = [
  "content-type",
  "content-length",
  "content-range",
  "accept-ranges",
  "x-content-type-options",
];

/**
 * Answer one request on the frame scheme.
 *
 * **The `Range` is passed through untouched and the answer is not rewritten.**
 * Fleet decides what a span comes back as — a 206 with its own `Content-Range`,
 * or a 416 naming the length — and a handler that composed either would be a
 * second opinion about a file it never read.
 *
 * Nothing connected is a 503 and a URL naming nothing is a 404, because a
 * `<video>` reads a status and there is no screen to put a sentence on.
 */
export function frameStream(port: PortOf, fetching: Fetching) {
  return async (request: Request): Promise<Response> => {
    const asked = askedFor(request.url);
    if (asked === null) return new Response(null, { status: 404 });
    const at = port();
    if (at === null) return new Response(null, { status: 503 });
    const range = request.headers.get("range");
    const path = asked.kept.split("/").map(encodeURIComponent).join("/");
    const to = `http://${HOST}:${at}/jobs/${encodeURIComponent(asked.jobId)}/frames/${path}`;
    try {
      const answer = await fetching(to, {
        method: "GET",
        ...(range === null ? {} : { headers: { range } }),
      });
      const headers = new Headers();
      for (const name of CARRIED) {
        const said = answer.headers.get(name);
        if (said !== null) headers.set(name, said);
      }
      return new Response(answer.body, { status: answer.status, headers });
    } catch {
      // **No timeout, and no retry.** A recording is read for as long as
      // somebody watches it, and a player that is told the read failed asks
      // again by itself.
      return new Response(null, { status: 502 });
    }
  };
}
