// Pulse's 10 s timer. The loop it runs on is pinned by `remarks-poll.test.ts`,
// which drives the same `JobPoll`; what is left to this file is what is Pulse's
// own — the interval, and that it stops with the board rather than the Job.

import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { PULSE_INTERVAL_MS } from "@armada/screens/src/pulse-poll";

import { ResourcesPoll } from "./resources-poll";

const A_JOB = "01M1HQZAKN001AJ5MT3PT09KKY";
const PORT = 4100;

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

it("takes the reading again while the board is open, on the interval the board says", async () => {
  const again = vi.fn(async () => {});
  const poll = new ResourcesPoll({ port: () => PORT, again });

  poll.watch(A_JOB);
  // A second short of the interval is a Job that has not been re-read yet: the
  // claim is the number Pulse prints, not "soon".
  await vi.advanceTimersByTimeAsync(PULSE_INTERVAL_MS - 1000);
  expect(again).not.toHaveBeenCalled();

  await vi.advanceTimersByTimeAsync(1000);
  expect(again).toHaveBeenCalledExactlyOnceWith(PORT, A_JOB);

  // Quiet Job, three more ticks: this is the whole point — nothing has moved
  // and the figures still do.
  await vi.advanceTimersByTimeAsync(PULSE_INTERVAL_MS * 3);
  expect(again).toHaveBeenCalledTimes(4);
});

it("stops when the board goes, not when the Job closes", async () => {
  const again = vi.fn(async () => {});
  const poll = new ResourcesPoll({ port: () => PORT, again });

  poll.watch(A_JOB);
  await vi.advanceTimersByTimeAsync(PULSE_INTERVAL_MS);
  expect(again).toHaveBeenCalledOnce();

  // Leaving the Pulse tab with the Job still open. **A process table every ten
  // seconds forever is what this is here to refuse**, so the interval is
  // cleared rather than left ticking against a `null`.
  poll.watch(null);
  await vi.advanceTimersByTimeAsync(PULSE_INTERVAL_MS * 10);
  expect(again).toHaveBeenCalledOnce();
});
