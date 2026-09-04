// What the Job's own log draws as, and what it must not lose on the way.
//
// Two of these are the issue's own warnings: attribution that goes missing, and
// a note that arrives as prose with nothing to open.

import { describe, expect, it } from "vitest";

import type { Journalled, Noted } from "@armada/protocol";
import { NOTHING_FROM_FLEET_YET, notesOf, whyNoNotes } from "./notes";

function noted(over: Partial<Noted> = {}): Noted {
  return {
    seq: 0,
    at: "2026-09-04T09:14:02.000Z",
    by: "fleet",
    level: "info",
    msg: "Worktree cut",
    ...over,
  };
}

describe("notesOf", () => {
  it("names who from the wire rather than from the socket it arrived on", () => {
    expect(notesOf([noted()])[0]?.actor).toBe("fleet");
  });

  it("opens to the fields the note carried", () => {
    const rows = notesOf([
      noted({
        fields: [
          { name: "branch", value: "armada/job_2d90bb" },
          { name: "at", value: ".armada/worktrees/job_2d90bb" },
        ],
      }),
    ]);
    expect(rows[0]?.payload.map((line) => line.text)).toEqual([
      "branch  armada/job_2d90bb",
      "at  .armada/worktrees/job_2d90bb",
    ]);
  });

  it("opens to nothing where the note carried nothing", () => {
    // Rather than to a sentence about itself. A row that offers to open
    // something that is not there is the false promise the log entry's own
    // chevron rule exists to prevent.
    expect(notesOf([noted()])[0]?.payload).toEqual([]);
  });

  it("names a failure and leaves an ordinary note as body", () => {
    const [failed, ordinary] = notesOf([
      noted({ level: "error", fields: [{ name: "exit", value: "1" }] }),
      noted({ seq: 1, fields: [{ name: "commands", value: "3" }] }),
    ]);
    expect(failed?.payload[0]?.named).toBe("failed");
    expect(ordinary?.payload[0]?.named).toBe("meta");
  });

  it("gives every row an id of its own, so two notes are two rows", () => {
    const rows = notesOf([noted(), noted({ seq: 1 })]);
    expect(new Set(rows.map((row) => row.id)).size).toBe(2);
  });
});

describe("whyNoNotes", () => {
  it("says nothing while the socket is reading", () => {
    const watching: Journalled = {
      state: "watching",
      jobId: "01JOB",
      log: { skipped: 0, notes: [] },
    };
    // **A Job Armada has done nothing to yet is not a failure**, and drawing a
    // reason there would tell somebody the log is broken when it is empty and
    // correct to be. The ordinary sentence is what fills that gap.
    expect(whyNoNotes(watching)).toBeUndefined();
    expect(NOTHING_FROM_FLEET_YET).toMatch(/not recorded anything/);
  });

  it("tells a log that could not be read from one that stopped", () => {
    const unreadable: Journalled = {
      state: "ended",
      jobId: "01JOB",
      log: { skipped: 0, notes: [] },
      because: "unreadable",
    };
    expect(whyNoNotes(unreadable)).toMatch(/could not read/);
    expect(whyNoNotes({ ...unreadable, because: "the connection closed" })).toMatch(
      /the connection closed/,
    );
  });
});
