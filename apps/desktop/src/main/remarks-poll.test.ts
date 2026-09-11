// The 20 s comments timer, pinned on its own rather than through
// `connection.test.ts`'s real Fleet double — a real 20 s wait has no place in
// a suite that runs on every commit, and none of this needs a socket.

import { afterEach, beforeEach, expect, it, vi } from "vitest";

import { INTERVAL_MS, RemarksPoll } from "./remarks-poll";

const A_JOB = "01M1HQZAKN001AJ5MT3PT09KKY";
const B_JOB = "01M1HQZAKN001AJ5MT3PT0OTHR";
const PORT = 4100;

/** A promise a test resolves by hand, to hold one tick's read open. */
function deferred<T>(): { promise: Promise<T>; resolve: (value: T) => void } {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((settle) => {
    resolve = settle;
  });
  return { promise, resolve };
}

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

it("ticks the Job on screen, and nothing once it leaves", async () => {
  const again = vi.fn(async () => {});
  const poll = new RemarksPoll({ port: () => PORT, again });

  poll.watch(A_JOB);
  await vi.advanceTimersByTimeAsync(INTERVAL_MS);
  expect(again).toHaveBeenCalledExactlyOnceWith(PORT, A_JOB);

  // Left the screen: the interval this started is cleared, not just skipped.
  poll.watch(null);
  await vi.advanceTimersByTimeAsync(INTERVAL_MS * 3);
  expect(again).toHaveBeenCalledOnce();
});

it("moves with the Job on screen, one at a time", async () => {
  const again = vi.fn(async () => {});
  const poll = new RemarksPoll({ port: () => PORT, again });

  poll.watch(A_JOB);
  poll.watch(B_JOB);
  await vi.advanceTimersByTimeAsync(INTERVAL_MS);
  // Only the Job held when the tick fires is asked for — B replaced A before
  // either got a tick of its own.
  expect(again).toHaveBeenCalledExactlyOnceWith(PORT, B_JOB);
});

it("skips a tick while the last one is still out, rather than queuing it", async () => {
  const first = deferred<void>();
  const again = vi.fn().mockReturnValueOnce(first.promise).mockResolvedValue(undefined);
  const poll = new RemarksPoll({ port: () => PORT, again });

  poll.watch(A_JOB);
  await vi.advanceTimersByTimeAsync(INTERVAL_MS);
  expect(again).toHaveBeenCalledTimes(1);

  // A second interval elapses with the first read still open.
  await vi.advanceTimersByTimeAsync(INTERVAL_MS);
  expect(again).toHaveBeenCalledTimes(1);

  // The first read lands, freeing the next tick to ask again.
  first.resolve();
  await vi.advanceTimersByTimeAsync(INTERVAL_MS);
  expect(again).toHaveBeenCalledTimes(2);
});

it("stops while the window is hidden, and resumes once it is shown again", async () => {
  const again = vi.fn(async () => {});
  const poll = new RemarksPoll({ port: () => PORT, again });

  poll.watch(A_JOB);
  poll.shown(false);
  await vi.advanceTimersByTimeAsync(INTERVAL_MS * 3);
  expect(again).not.toHaveBeenCalled();

  poll.shown(true);
  await vi.advanceTimersByTimeAsync(INTERVAL_MS);
  expect(again).toHaveBeenCalledExactlyOnceWith(PORT, A_JOB);
});

it("asks nothing where no Job is on screen, however long it runs", async () => {
  const again = vi.fn(async () => {});
  const poll = new RemarksPoll({ port: () => PORT, again });

  poll.shown(true);
  await vi.advanceTimersByTimeAsync(INTERVAL_MS * 5);
  expect(again).not.toHaveBeenCalled();
});
