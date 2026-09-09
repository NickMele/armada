// Which of two answers about one Job a surface keeps.
//
// **A real server on a loopback port rather than a stub for `ask`**, for
// `journal.test.ts`'s reason: what is under test is which answer survives a
// round trip, and a stub that resolved in call order would decide the thing the
// case is about. This server holds both answers open and releases them in the
// order the case names, which is the only way to write "the first read came
// back last" down.

import { createServer, type Server } from "node:http";
import { once } from "node:events";
import type { AddressInfo } from "node:net";

import { afterEach, expect, it } from "vitest";

import type { JobRead } from "@armada/protocol";
import { JobReader } from "./reader";
import { HOST } from "./runtime-file";

const A_JOB = "01M21BKVPW002DC0ATD1X9T0VF";
const ANOTHER_JOB = "01M1HQZAKN001AJ5MT3PT09KKY";

/** Everything one case opens, closed in the order it was opened. */
const opened: (() => void)[] = [];

afterEach(() => {
  while (opened.length > 0) opened.pop()?.();
});

/** One request the server is holding, and the two ways it may end. */
type Held = { answer: (body: unknown) => void; breaks: () => void };

/**
 * A Fleet that answers nothing until a case says so.
 *
 * Every request is parked and handed back in arrival order, so a case can
 * answer the second before the first — which is the interleaving this file is
 * about and the one a stub cannot produce.
 */
async function holding(): Promise<{ port: number; held: Held[] }> {
  const held: Held[] = [];
  const server: Server = createServer((_, response) => {
    held.push({
      answer: (body) => {
        response.writeHead(200, { "content-type": "application/json" });
        response.end(JSON.stringify(body));
      },
      // Cut mid-request, which is what `ask` reports as a transport failure —
      // the shape a timed-out read arrives in, without waiting five seconds
      // for one.
      breaks: () => response.destroy(),
    });
  });
  server.listen(0, HOST);
  await once(server, "listening");
  opened.push(() => server.close());
  return { port: (server.address() as AddressInfo).port, held };
}

/** A reader that keeps every state it published. */
function reading(): { reader: JobReader<{ seen: unknown }>; states: JobRead<{ seen: unknown }>[] } {
  const states: JobRead<{ seen: unknown }>[] = [];
  const reader = new JobReader<{ seen: unknown }>({
    route: (jobId) => `/jobs/${jobId}`,
    keeps: (body) => ({ seen: body }),
    publish: (state) => states.push(state),
  });
  return { reader, states };
}

/** Poll until the server is holding `how_many` requests, or give up. */
async function parked(held: Held[], howMany: number): Promise<void> {
  for (let n = 0; n < 200; n += 1) {
    if (held.length >= howMany) return;
    await new Promise((wake) => setTimeout(wake, 5));
  }
  throw new Error(`only ${held.length} of ${howMany} requests arrived`);
}

/** What the reader is showing, of the states it published. */
function shown(states: JobRead<{ seen: unknown }>[]): unknown {
  const last = states[states.length - 1];
  return last?.state === "read" ? last.seen : last?.state;
}

/**
 * **The newest read wins, whichever answer lands first.**
 *
 * Job `01M21BKVPW002DC0ATD1X9T0VF`: a step boundary publishes a Drone exiting
 * and a step advancing about twenty milliseconds apart, and both re-read the
 * open Job. Each caller is `void`-ed, so the two overlap and whichever response
 * lands last used to be what the screen kept — which is how an eighteen-minute
 * old spend survived the event that should have corrected it.
 *
 * The first request is answered second here, with the figure it would have had
 * before the boundary.
 */
it("keeps the newest read when an older one answers after it", async () => {
  const { port, held } = await holding();
  const { reader, states } = reading();

  const first = reader.want(port, A_JOB);
  await parked(held, 1);
  const later = reader.again(port);
  await parked(held, 2);

  held[1]?.answer({ cost_micros: 5284509 });
  held[0]?.answer({ cost_micros: 4121781 });
  await Promise.all([first, later]);

  expect(shown(states)).toEqual({ cost_micros: 5284509 });
});

/**
 * **The pair to it**: answered in the order they were asked, the newest still
 * wins. Without this the case above passes against a reader that simply kept
 * the first answer and dropped every later one.
 */
it("keeps the newest read when the answers arrive in order", async () => {
  const { port, held } = await holding();
  const { reader, states } = reading();

  const first = reader.want(port, A_JOB);
  await parked(held, 1);
  const later = reader.again(port);
  await parked(held, 2);

  held[0]?.answer({ cost_micros: 4121781 });
  held[1]?.answer({ cost_micros: 5284509 });
  await Promise.all([first, later]);

  expect(shown(states)).toEqual({ cost_micros: 5284509 });
});

/**
 * **A stale answer does not blank a panel the newer read is about to fill.**
 *
 * The drop covers a failure as much as a success: a reader that dropped only
 * the stale `read` would still let a stale timeout publish `failed` over a Job
 * that is answering perfectly well.
 */
it("drops an older read that failed rather than showing its failure", async () => {
  const { port, held } = await holding();
  const { reader, states } = reading();

  const first = reader.want(port, A_JOB);
  await parked(held, 1);
  const later = reader.again(port);
  await parked(held, 2);

  held[1]?.answer({ cost_micros: 5284509 });
  await later;
  held[0]?.breaks();
  await first;

  expect(shown(states)).toEqual({ cost_micros: 5284509 });
});

/**
 * **The rule this file already had, still true.** An answer for a Job nobody
 * has open is dropped, which is what stops one Job's read painting the Job that
 * replaced it — a different failure from the one above, and both are needed.
 */
it("drops an answer whose Job was replaced while it was in flight", async () => {
  const { port, held } = await holding();
  const { reader, states } = reading();

  const first = reader.want(port, A_JOB);
  await parked(held, 1);
  const moved = reader.want(port, ANOTHER_JOB);
  await parked(held, 2);

  held[0]?.answer({ cost_micros: 4121781 });
  held[1]?.answer({ cost_micros: 0 });
  await Promise.all([first, moved]);

  const painted = states.filter((state) => state.state === "read");
  expect(painted.every((state) => state.jobId === ANOTHER_JOB)).toBe(true);
});
