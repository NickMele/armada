// What a window may ask for on the frame scheme, and what main does with it.

// **The subject is the address.** Everything else here is one fetch; what is
// worth pinning is that the only thing a page can name is a Job and a frame id
// the record already gave out, that the range crosses untouched, and that no
// port is invented when nothing is connected. That is the difference between a
// stream and an arbitrary URL handler, and it is the kind of comparison that
// gets deleted as redundant by somebody who did not know why it was there.

import { describe, expect, it, vi } from "vitest";

import { askedFor, frameStreamUrl } from "../shared/streaming";
import { frameStream, type Fetching } from "./streaming";

const JOB = "01M1N1TJB3002E49K150S7AF2B";
const KEPT = "implement.1.branch/walkthrough.webm";

/** A Fleet that answers a span, and writes down what it was asked. */
function answering(): { fetching: Fetching; calls: { url: string; range?: string }[] } {
  const calls: { url: string; range?: string }[] = [];
  const fetching: Fetching = async (url, init) => {
    const range = init.headers?.["range"];
    calls.push({ url, ...(range === undefined ? {} : { range }) });
    return new Response("efghi", {
      status: 206,
      headers: {
        "content-type": "video/webm",
        "content-range": "bytes 4-8/26",
        "accept-ranges": "bytes",
        "x-content-type-options": "nosniff",
        "set-cookie": "not carried",
      },
    });
  };
  return { fetching, calls };
}

describe("what an address on the frame scheme names", () => {
  it("reads back the Job and the frame id this app composed", () => {
    expect(askedFor(frameStreamUrl(JOB, KEPT))).toEqual({ jobId: JOB, kept: KEPT });
  });

  it("survives a name with a space in it, which a spec may well choose", () => {
    const kept = "implement.1.branch/a walk through.webm";
    expect(askedFor(frameStreamUrl(JOB, kept))).toEqual({ jobId: JOB, kept });
  });

  /**
   * **A frame id is a run and a file, and nothing else is one.** Fleet resolves
   * every name against the record before it opens anything, so this is the
   * cheap half of that rule rather than the only one — but a scheme that
   * accepted a climb would be a hole with somebody else's guard behind it.
   */
  it("names nothing for an address this app would not have written", () => {
    for (const url of [
      "https://forge.invalid/jobs/01JOB/frames/show.1/home.webm",
      "armada-frame://elsewhere/01JOB/show.1/home.webm",
      "armada-frame://frame/01JOB/show.1",
      "armada-frame://frame/01JOB/show.1/home.webm/extra",
      "armada-frame://frame/01JOB/../../etc/passwd",
      `armada-frame://frame/01JOB/show.1/${encodeURIComponent("../../etc/passwd")}`,
      "not a url at all",
    ]) {
      expect(askedFor(url), url).toBeNull();
    }
  });
});

describe("forwarding a recording", () => {
  it("carries the range out and the answer back, and adds nothing of its own", async () => {
    const { fetching, calls } = answering();
    const handler = frameStream(() => 4711, fetching);
    const answer = await handler(
      new Request(frameStreamUrl(JOB, KEPT), { headers: { range: "bytes=4-8" } }),
    );

    expect(calls).toEqual([
      {
        url: `http://127.0.0.1:4711/jobs/${JOB}/frames/implement.1.branch/walkthrough.webm`,
        range: "bytes=4-8",
      },
    ]);
    expect(answer.status).toBe(206);
    expect(answer.headers.get("content-range")).toBe("bytes 4-8/26");
    expect(answer.headers.get("content-type")).toBe("video/webm");
    expect(answer.headers.get("x-content-type-options")).toBe("nosniff");
    // The hop's own headers stay on the hop.
    expect(answer.headers.get("set-cookie")).toBeNull();
    await expect(answer.text()).resolves.toBe("efghi");
  });

  it("asks for the whole file when the player did not ask for a span", async () => {
    const { fetching, calls } = answering();
    const handler = frameStream(() => 4711, fetching);
    await handler(new Request(frameStreamUrl(JOB, KEPT)));
    expect(calls[0]?.range).toBeUndefined();
  });

  /**
   * **Nothing connected invents no port.** The renderer reaching Fleet is the
   * thing this whole path exists to avoid, and a handler that guessed a port
   * would be that, one process further in.
   */
  it("answers that there is nothing to ask when no Fleet is connected", async () => {
    const fetching = vi.fn<Fetching>();
    const handler = frameStream(() => null, fetching);
    const answer = await handler(new Request(frameStreamUrl(JOB, KEPT)));
    expect(answer.status).toBe(503);
    expect(fetching).not.toHaveBeenCalled();
  });

  it("sends nothing at all for an address that names nothing", async () => {
    const fetching = vi.fn<Fetching>();
    const handler = frameStream(() => 4711, fetching);
    const answer = await handler(new Request("armada-frame://frame/01JOB/climb/../../etc"));
    expect(answer.status).toBe(404);
    expect(fetching).not.toHaveBeenCalled();
  });

  it("says the read failed rather than throwing into the window", async () => {
    const fetching: Fetching = async () => {
      throw new Error("connection refused");
    };
    const handler = frameStream(() => 4711, fetching);
    const answer = await handler(new Request(frameStreamUrl(JOB, KEPT)));
    expect(answer.status).toBe(502);
  });
});
