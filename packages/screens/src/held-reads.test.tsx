// The holding every fetched-on-demand read shares, with no drawing on top.
//
// **What is checked is the three rules the copies each had to keep**: ask once
// per key, hold nothing across a Job, and never hold a late answer under the
// Job that replaced the one it was for. The last is new — each copy reset its
// map with an effect and then let an answer still in flight write straight
// back into it.
import { afterEach, expect, test, vi } from "vitest";
import { page } from "vitest/browser";
import { useEffect } from "react";

import { useHeldReads } from "./held-reads";
import { mount, rerender, unmount } from "./mounted";

afterEach(unmount);

type State = { state: "fetching" } | { state: "got"; text: string } | { state: "absent" };

const SETTLE = (text: string): State => ({ state: "got", text });
const ASKING: State = { state: "fetching" };
const FAILED: State = { state: "absent" };

function Probe({ read, jobId, keys }: { read: (jobId: string, key: string) => Promise<string>; jobId: string; keys: string[] }) {
  const held = useHeldReads({ read, jobId, settle: SETTLE, asking: ASKING, failed: FAILED });
  useEffect(() => {
    for (const key of keys) held.fetch(key);
  });
  return (
    <ul aria-label="held">
      {keys.map((key) => {
        const state = held.of(key);
        return (
          <li key={key}>
            {key}: {state === undefined ? "unasked" : state.state === "got" ? state.text : state.state}
          </li>
        );
      })}
    </ul>
  );
}

test("asks once per key, however often it is rendered", async () => {
  const read = vi.fn(async (_job: string, key: string) => `read ${key}`);
  mount(<Probe read={read} jobId="01JOB" keys={["a"]} />);
  await expect.element(page.getByText("a: read a")).toBeInTheDocument();

  rerender(<Probe read={read} jobId="01JOB" keys={["a"]} />);
  rerender(<Probe read={read} jobId="01JOB" keys={["a"]} />);
  await expect.element(page.getByText("a: read a")).toBeInTheDocument();
  expect(read).toHaveBeenCalledTimes(1);
});

test("reads as nothing under the next Job, and asks again for it", async () => {
  const read = vi.fn(async (job: string, key: string) => `${job} ${key}`);
  mount(<Probe read={read} jobId="01JOB" keys={["a"]} />);
  await expect.element(page.getByText("a: 01JOB a")).toBeInTheDocument();

  rerender(<Probe read={read} jobId="01NEXT" keys={["a"]} />);
  await expect.element(page.getByText("a: 01NEXT a")).toBeInTheDocument();
  expect(read).toHaveBeenCalledTimes(2);
});

test("drops an answer that lands after its Job was replaced", async () => {
  let answerFirst: (text: string) => void = () => {};
  const read = vi.fn((job: string, _key: string) =>
    job === "01JOB"
      ? new Promise<string>((resolve) => {
          answerFirst = resolve;
        })
      : new Promise<string>(() => {}),
  );
  mount(<Probe read={read} jobId="01JOB" keys={["a"]} />);
  await expect.element(page.getByText("a: fetching")).toBeInTheDocument();

  rerender(<Probe read={read} jobId="01NEXT" keys={["a"]} />);
  await expect.element(page.getByText("a: fetching")).toBeInTheDocument();

  // The first Job's answer arrives now, and it is not the next Job's to draw.
  answerFirst("the first Job's file");
  await new Promise((settled) => setTimeout(settled, 50));
  await expect.element(page.getByText("a: fetching")).toBeInTheDocument();
});

test("holds a rejected round trip as the failure, not as still asking", async () => {
  const read = vi.fn(async () => {
    throw new Error("main is gone");
  });
  mount(<Probe read={read} jobId="01JOB" keys={["a"]} />);
  await expect.element(page.getByText("a: absent")).toBeInTheDocument();
});
