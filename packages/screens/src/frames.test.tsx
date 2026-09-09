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

import { useFrames, type ReadFrame } from "./frames";
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
  const frames = useFrames(read, jobId);
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
                ? `got ${state.src.startsWith("blob:") ? "a blob" : state.src}`
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
