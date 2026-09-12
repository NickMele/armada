// Holding a Job's frames, mounted — because the half worth proving is a hook.
//
// **The revoking is the subject.** An object URL is held by the document until
// it is revoked, so a reader walking six Jobs leaves six sets of screenshots in
// memory: a leak nothing on screen shows, no type catches and no function test
// can reach, because the release is an effect's cleanup. That is why this is a
// browser test rather than arithmetic, and why the assertions are counts of
// what was minted against what was given back.
//
// `URL.createObjectURL` here is the browser's own. Only `revokeObjectURL` is
// wrapped, and only to count — a stub for both would prove this file's
// bookkeeping rather than the browser's.

import { useEffect } from "react";
import { afterEach, beforeEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";

import type { FrameRead, KeptFrame } from "@armada/protocol";

import { useFrames, type FrameSrc, type ReadFrame } from "./frames";
import { mount, unmount } from "./mounted";

function frame(over: Partial<KeptFrame> = {}): KeptFrame {
  return {
    attempt: 1,
    name: "home.png",
    path: ".armada/frames/12-a-job/show.1/home.png",
    bytes: 41_002,
    kept: "show.1/home.png",
    ...over,
  };
}

/** What Fleet answers with. Eight bytes of PNG signature and nothing else. */
const got: FrameRead = {
  ok: true,
  bytes: new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
  type: "image/png",
};

/**
 * Where a recording streams from, spelled the way the app spells it.
 *
 * **Module scope, because the hook depends on it.** One rebuilt per render
 * would make `want` a new function every time, which is the loop the stable
 * host calls exist to prevent.
 */
const streamed: FrameSrc = (jobId, kept) => `armada-frame://frame/${jobId}/${kept}`;

/** Every URL handed back, counted. The minting stays the browser's own. */
let revoked: string[] = [];
const real = URL.revokeObjectURL;

beforeEach(() => {
  revoked = [];
  URL.revokeObjectURL = (src: string) => {
    revoked.push(src);
    real.call(URL, src);
  };
});

afterEach(() => {
  URL.revokeObjectURL = real;
  unmount();
});

/**
 * A probe: the hook, asking for what it is given and writing down what it has.
 *
 * **No component of the app**, because what is under test is the holding and
 * not the drawing — `FramesShown` has its own stories for that, and mounting it
 * here would make a failure ambiguous between the two.
 */
function Probe({ read, jobId, rows }: { read: ReadFrame; jobId: string; rows: KeptFrame[] }) {
  const frames = useFrames(read, jobId, streamed);
  useEffect(() => {
    if (rows.length > 0) frames.want(rows);
  }, [rows, frames]);
  return (
    <ul aria-label="held">
      {rows.map((row) => {
        const state = frames.of(row.kept);
        return (
          <li key={row.kept} data-kept={row.kept}>
            {state === undefined
              ? "unasked"
              : state.state === "got"
                ? state.content.kind === "image" || state.content.kind === "video"
                  ? `got ${state.content.src.startsWith("blob:") ? "a blob" : state.content.src}`
                  : `got text: ${state.content.text}`
                : state.state === "absent"
                  ? state.note
                  : "fetching"}
          </li>
        );
      })}
    </ul>
  );
}

test("asks once per frame, however often the story is rebuilt", async () => {
  const read = vi.fn(async () => got);
  const rows = [frame()];
  mount(<Probe read={read} jobId="01JOB" rows={rows} />);
  await expect.element(page.getByText("got a blob")).toBeInTheDocument();

  // The story is rebuilt on every tick of the clock. The same rows going in
  // again must not send a second request for a file already held.
  mount(<Probe read={read} jobId="01JOB" rows={rows} />);
  await expect.element(page.getByText("got a blob")).toBeInTheDocument();

  // Two mounts, and the second holds nothing — a fresh hook is a fresh Job's
  // worth of state. What is asserted is that neither mount asked twice.
  expect(read).toHaveBeenCalledTimes(2);
});

test("asks for the frames it has not seen and leaves the rest alone", async () => {
  const read = vi.fn(async () => got);
  const settings = frame({ kept: "show.1/settings.png", name: "settings.png" });
  mount(<Probe read={read} jobId="01JOB" rows={[frame(), settings]} />);

  await expect.element(page.getByText("got a blob").first()).toBeInTheDocument();
  expect(read.mock.calls.map((call) => call[1]).sort()).toEqual([
    "show.1/home.png",
    "show.1/settings.png",
  ]);
});

/**
 * **The leak this file exists for.** A `kept` names a file under the Job whose
 * `.armada/frames` holds it, so a new Job drops what was held — and every URL
 * minted for the old one has to go with it, or the document keeps the bytes.
 */
test("gives every URL back when the Job changes", async () => {
  const read = vi.fn(async () => got);
  mount(<Probe read={read} jobId="01JOB" rows={[frame(), frame({ kept: "show.1/b.png" })]} />);
  await expect.element(page.getByText("got a blob").first()).toBeInTheDocument();
  expect(revoked).toHaveLength(0);

  mount(<Probe read={read} jobId="01OTHER" rows={[frame()]} />);
  await expect.element(page.getByText("got a blob").first()).toBeInTheDocument();
  // The two the first Job minted, given back. The new Job's own is not among
  // them — it is still on screen.
  expect(revoked).toHaveLength(2);
});

test("gives every URL back when the window goes", async () => {
  const read = vi.fn(async () => got);
  mount(<Probe read={read} jobId="01JOB" rows={[frame()]} />);
  await expect.element(page.getByText("got a blob")).toBeInTheDocument();

  unmount();
  expect(revoked).toHaveLength(1);
});

/**
 * A refusal on this route is the row being on the record and the file not — a
 * reclaimed `.armada/frames`. That is a sentence about one frame, and the
 * others of its run are still drawn.
 */
test("says which silence a frame that would not come back is", async () => {
  const read: ReadFrame = async (_job, kept) =>
    kept === "show.1/home.png"
      ? got
      : {
          ok: false,
          outcome: {
            ok: false,
            why: "refused",
            error: {
              code: "fleet.no_such_frame",
              message: "no step of this Job kept a frame named `show.1/settings.png`",
              run_id: "01RUN",
            },
          },
        };
  mount(
    <Probe
      read={read}
      jobId="01JOB"
      rows={[frame(), frame({ kept: "show.1/settings.png", name: "settings.png" })]}
    />,
  );

  await expect
    .element(page.getByText("This frame is on the record and no longer on disk."))
    .toBeInTheDocument();
  // The one that did come back is still held. A failed read is one frame's
  // business, never the list's.
  await expect.element(page.getByText("got a blob")).toBeInTheDocument();
});

test("tells a Fleet that did not answer apart from a file that is gone", async () => {
  const read: ReadFrame = async () => ({
    ok: false,
    outcome: { ok: false, why: "not_connected" },
  });
  mount(<Probe read={read} jobId="01JOB" rows={[frame()]} />);

  await expect
    .element(page.getByText("Fleet did not answer for this frame."))
    .toBeInTheDocument();
});

/**
 * **Text and JSON decode to a string, never a `blob:` URL.** There is nothing
 * to revoke for either — the whole reason this module holds an object URL at
 * all is that an `<img>` needs one, and a `<pre>` reads a string directly.
 */
test("decodes text and JSON rather than minting a URL for them", async () => {
  const read: ReadFrame = async (_job, kept) =>
    kept === "implement.1/plan.txt"
      ? { ok: true, bytes: new TextEncoder().encode("do the thing"), type: "text/plain" }
      : { ok: true, bytes: new TextEncoder().encode('{"ok":true}'), type: "application/json" };
  mount(
    <Probe
      read={read}
      jobId="01JOB"
      rows={[
        frame({ kept: "implement.1/plan.txt", name: "plan.txt" }),
        frame({ kept: "implement.1/result.json", name: "result.json" }),
      ]}
    />,
  );
  await expect.element(page.getByText("got text: do the thing")).toBeInTheDocument();
  await expect.element(page.getByText('got text: {"ok":true}')).toBeInTheDocument();
  expect(revoked).toHaveLength(0);
});

/**
 * **A kind Fleet answered as bytes and nothing here can place draws as a
 * refusal, not a broken plate.** `application/octet-stream` is what Fleet
 * sends for an SVG or an HTML file on purpose, and for anything else
 * `answers::media_type` does not name.
 */
test("says it does not know how to draw a kind it cannot place", async () => {
  const read: ReadFrame = async () => ({
    ok: true,
    bytes: new Uint8Array([1, 2, 3]),
    type: "application/octet-stream",
  });
  mount(<Probe read={read} jobId="01JOB" rows={[frame({ name: "report.pdf" })]} />);
  await expect
    .element(page.getByText("Bridge does not know how to draw this kind of file."))
    .toBeInTheDocument();
});

/**
 * **A recording is never read, whatever it weighs.** The twenty-mebibyte skip
 * this replaced was a decision made off `KeptFrame.bytes`; there is no size
 * here to decide anything, because what a plate gets is an address main
 * streams a span at a time. Proved by asserting `read` was never called and
 * that what is held is that address rather than a `blob:`.
 */
test("points at a video rather than reading it, however large it is", async () => {
  const read = vi.fn(async () => got);
  const heavy = frame({
    kept: "implement.1/walkthrough.webm",
    name: "walkthrough.webm",
    bytes: 900 * 1024 * 1024,
  });
  mount(<Probe read={read} jobId="01JOB" rows={[heavy]} />);
  await expect
    .element(page.getByText("got armada-frame://frame/01JOB/implement.1/walkthrough.webm"))
    .toBeInTheDocument();
  expect(read).not.toHaveBeenCalled();
  // Nothing was minted for it, so there is nothing to give back either.
  expect(revoked).toHaveLength(0);
});

/** A small one goes the same way: the size decides nothing any more. */
test("points at a small recording too", async () => {
  const read = vi.fn(async () => got);
  const light = frame({
    kept: "implement.1/walkthrough.webm",
    name: "walkthrough.webm",
    bytes: 1024,
  });
  mount(<Probe read={read} jobId="01JOB" rows={[light]} />);
  await expect
    .element(page.getByText("got armada-frame://frame/01JOB/implement.1/walkthrough.webm"))
    .toBeInTheDocument();
  expect(read).not.toHaveBeenCalled();
});

/**
 * A recording beside an image: one is pointed at and the other is fetched, in
 * the same pass over the same step's rows.
 */
test("asks only for what it does not stream", async () => {
  const read = vi.fn(async () => got);
  const recording = frame({
    kept: "implement.1/walkthrough.webm",
    name: "walkthrough.webm",
    bytes: 41_002,
  });
  mount(<Probe read={read} jobId="01JOB" rows={[frame(), recording]} />);
  await expect.element(page.getByText("got a blob")).toBeInTheDocument();
  expect(read.mock.calls.map((call) => call[1])).toEqual(["show.1/home.png"]);
});
