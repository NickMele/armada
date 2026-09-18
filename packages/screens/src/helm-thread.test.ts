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

  it("says why a stored session could not be resumed", () => {
    const fresh: HelmThreadItem = { kind: "fresh", id: "4", ts: "2026-09-13T10:00:00.000Z", because: "session_not_found" };
    expect(helmRowsOf([fresh])[0]?.message).toMatch(/could not be resumed/);
  });

  // The row drew `No reply came. ${why}` until the reply budget's own sentence
  // started with the same words, so a person read it twice. Fleet writes the
  // whole sentence now — `crates/fleet/src/helm/unanswered.rs`, pinned on
  // Fleet's side by `crates/fleet/src/tests/helm_unanswered.rs` — and the
  // record draws the same three in `HelmRecord.stories.tsx`.
  it.each([
    "no reply came within 900 seconds, so the session was ended",
    "the session never started: the model provider refused this key (exit status: 1)",
    "the conversation's stored session could not be reached: database is locked",
  ])("draws Fleet's own sentence for a reply that never came: %s", (why) => {
    const unanswered: HelmThreadItem = { kind: "unanswered", id: "5", ts: "2026-09-13T10:00:00.000Z", why };
    const [row] = helmRowsOf([unanswered]);
    expect(row?.message).toBe(why);
    expect(String(row?.message).match(/no reply came/gi) ?? []).toHaveLength(
      why.includes("no reply came") ? 1 : 0,
    );
    expect(row?.message).not.toMatch(/^No reply came\. /);
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

  it("folds an approval ask's called row to the open reply's asks, and leaves every other called row drawing nothing", () => {
    // The real shape: a session's own tool names arrive prefixed by the MCP
    // server, `mcp__armada-fleet__...` — never bare. #1041's own defect: a
    // fold matching the bare name alone drew nothing in the real app.
    const ask = called("a", "mcp__armada-fleet__ask_person_to_approve", "01JOB9");
    const other = called("b", "mcp__armada-fleet__get_job", "job_id=12");
    const reply: HelmThreadItem = {
      kind: "row",
      id: "c",
      turn: { ts: "2026-09-13T10:00:02.000Z", seq: 2, by: "drone", saw: { event: "said", text: "Here is the card." } },
    };
    const rows = helmRowsOf([ASKED, other, ask, reply]);
    expect(rows).toHaveLength(2);
    expect(rows[1]).toMatchObject({
      message: "Here is the card.",
      asks: [{ id: "a", jobId: "01JOB9" }],
    });
  });

  it("opens a reply for an approval ask that arrives with no said text of its own", () => {
    const rows = helmRowsOf([ASKED, called("a", "mcp__armada-fleet__ask_person_to_approve", "01JOB9"), ENDED]);
    expect(rows).toHaveLength(2);
    expect(rows[1]).toMatchObject({ asks: [{ id: "a", jobId: "01JOB9" }] });
  });

  it("also folds the bare tool name, the inventory's own spelling", () => {
    const rows = helmRowsOf([ASKED, called("a", "ask_person_to_approve", "01JOB9"), ENDED]);
    expect(rows).toHaveLength(2);
    expect(rows[1]).toMatchObject({ asks: [{ id: "a", jobId: "01JOB9" }] });
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
