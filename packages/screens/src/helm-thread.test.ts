// What `helm-thread.ts` draws from Helm's own wire — #944.

import { describe, expect, it } from "vitest";

import type { HelmThreadItem } from "@armada/protocol";
import { helmRowsOf } from "./helm-thread";

const ASKED: HelmThreadItem = { kind: "asked", id: "0", ts: "2026-09-13T10:00:00.000Z", text: "Why did job 12 stall?" };
const SAID: HelmThreadItem = {
  kind: "row",
  id: "1",
  turn: { ts: "2026-09-13T10:00:02.000Z", seq: 1, by: "drone", saw: { event: "said", text: "It is waiting on a command." } },
};
const ENDED: HelmThreadItem = {
  kind: "row",
  id: "2",
  turn: { ts: "2026-09-13T10:00:03.000Z", seq: 2, by: "drone", saw: { event: "ended", turns: 3, cost_micros: 12_000, refusals: 0 } },
};
const STARTED: HelmThreadItem = {
  kind: "row",
  id: "3",
  turn: { ts: "2026-09-13T10:00:01.000Z", seq: 0, by: "drone", saw: { event: "started", session: "s", model: "m", mcp_servers: 0 } },
};

/** One `unrecognised` row, `id` free so a run of them can be told apart. */
function unrecognised(id: string): HelmThreadItem {
  return {
    kind: "row",
    id,
    turn: { ts: "2026-09-13T23:02:42.000Z", seq: Number(id), by: "drone", saw: { event: "unrecognised", kind: "stream_event" } },
  };
}

/** One `called` row — a tool Helm reached for along the way. */
function called(id: string, tool: string, detail: string): HelmThreadItem {
  return {
    kind: "row",
    id,
    turn: {
      ts: "2026-09-13T23:02:43.000Z",
      seq: Number(id),
      by: "drone",
      saw: { event: "called", tool, call: id, detail, truncated: false },
    },
  };
}

describe("helmRowsOf", () => {
  it("reads a person's own words as You, and the session's as Helm", () => {
    const rows = helmRowsOf([ASKED, SAID]);
    expect(rows).toMatchObject([
      { actor: "you", message: "Why did job 12 stall?" },
      { actor: "helm", message: "It is waiting on a command." },
    ]);
  });

  it("draws a reply's ended row as a quiet cost line under it, not a bubble of its own", () => {
    const rows = helmRowsOf([ASKED, SAID, ENDED]);
    expect(rows).toHaveLength(2);
    expect(rows.at(-1)).toMatchObject({ actor: "helm", message: "It is waiting on a command.", meta: "~$0.01 · 3 turns" });
  });

  it("leaves out what a person watching a chat has no use for", () => {
    const rows = helmRowsOf([STARTED, ASKED, SAID]);
    expect(rows).toHaveLength(2);
  });

  it("says why a stored session could not be resumed, and why no reply came", () => {
    const fresh: HelmThreadItem = { kind: "fresh", id: "4", ts: "2026-09-13T10:00:00.000Z", because: "session_not_found" };
    const unanswered: HelmThreadItem = { kind: "unanswered", id: "5", ts: "2026-09-13T10:00:00.000Z", why: "Fleet is shutting down." };
    const rows = helmRowsOf([fresh, unanswered]);
    expect(rows[0]?.message).toMatch(/could not be resumed/);
    expect(rows[1]?.message).toMatch(/Fleet is shutting down\./);
  });

  it("folds the owner's own sequence to their question, one reply, and its cost", () => {
    // What #1030's dock actually drew: an `asked`, five `unrecognised` before
    // the model ever called anything, three tool calls, five more
    // `unrecognised`, the reply, then `ended` — thirteen rows the person read
    // as "Helm unrecognised" five times over, twice.
    const rows = helmRowsOf([
      ASKED,
      unrecognised("u1"),
      unrecognised("u2"),
      unrecognised("u3"),
      unrecognised("u4"),
      unrecognised("u5"),
      called("c1", "ToolSearch", "select:mcp__armada-fleet__get_events_since"),
      called("c2", "mcp__armada-fleet__get_events_since", ""),
      called("c3", "mcp__armada-fleet__get_job", ""),
      unrecognised("u6"),
      unrecognised("u7"),
      unrecognised("u8"),
      unrecognised("u9"),
      unrecognised("u10"),
      {
        kind: "row",
        id: "reply",
        turn: {
          ts: "2026-09-13T23:02:46.000Z",
          seq: 20,
          by: "drone",
          saw: {
            event: "said",
            text:
              "No — approving a Job isn't one of the acts I can take. Job 9 (`9-fix-801-unanswered-permission-ask-holds-dr`) is `awaiting_approval` with no steps started yet; approval is yours to give in Bridge.",
          },
        },
      },
      {
        kind: "row",
        id: "cost",
        turn: {
          ts: "2026-09-13T23:02:46.000Z",
          seq: 21,
          by: "drone",
          saw: { event: "ended", turns: 4, cost_micros: 130_000, refusals: 0 },
        },
      },
    ]);

    expect(rows).toHaveLength(2);
    expect(rows[0]).toMatchObject({ actor: "you", message: "Why did job 12 stall?" });
    expect(rows[1]).toMatchObject({
      actor: "helm",
      message: expect.stringContaining("approving a Job isn't one of the acts"),
      meta: "~$0.13 · 4 turns",
    });
  });

  it("reads several said turns, with a tool call between them, as one reply", () => {
    const first: HelmThreadItem = {
      kind: "row",
      id: "a",
      turn: { ts: "2026-09-13T10:00:02.000Z", seq: 1, by: "drone", saw: { event: "said", text: "Checking the Job now." } },
    };
    const tool = called("b", "mcp__armada-fleet__get_job", "job_id=12");
    const second: HelmThreadItem = {
      kind: "row",
      id: "c",
      turn: { ts: "2026-09-13T10:00:04.000Z", seq: 3, by: "drone", saw: { event: "said", text: "It is waiting on a command." } },
    };
    const rows = helmRowsOf([ASKED, first, tool, second]);
    expect(rows).toHaveLength(2);
    expect(rows[1]?.message).toBe("Checking the Job now.\n\nIt is waiting on a command.");
  });
});
