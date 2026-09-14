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

describe("helmRowsOf", () => {
  it("reads a person's own words as You, and the session's as Helm", () => {
    const rows = helmRowsOf([ASKED, SAID]);
    expect(rows).toMatchObject([
      { actor: "you", message: "Why did job 12 stall?" },
      { actor: "helm", message: "It is waiting on a command." },
    ]);
  });

  it("draws a reply's ended row as what it cost", () => {
    const rows = helmRowsOf([ASKED, SAID, ENDED]);
    expect(rows.at(-1)).toMatchObject({ actor: "helm", message: "~$0.01 · 3 turns" });
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
});
