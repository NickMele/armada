// What a stopped Job says it was refused, and the two ways that sentence used
// to be wrong.
//
// **A Job refused nothing must draw nothing**, because most stopped Jobs were
// refused nothing and a heading over an empty list on every one of them says a
// policy was involved where none was.
//
// **A capped list must say it is capped.** A short list read as the whole one
// is worse than no list: a person who widened an allowlist for the fifty they
// could see would think they were done.

import { describe, expect, it } from "vitest";

import type { JobDetail as JobWhole, Refusal, Stuck } from "@armada/protocol";
import { refusedIn } from "./refused";

/** One refusal, as the wire carries it: no reason, and the command on `detail`. */
function refusal(over: Partial<Refusal> = {}): Refusal {
  return {
    tool: "Bash",
    call: "toolu_01ANq8Yk3sWnQ2gVv7hHc4Pd",
    detail: "cargo nextest run --package ipc 2>&1 | tail -80",
    truncated: false,
    length: 47,
    because: "",
    ...over,
  };
}

/** A stopped Job, classified, with whatever it was refused. */
function whole(over: Partial<Stuck> = {}): JobWhole {
  return {
    job: {
      id: "01M130Y1380016YK5S0JXBXDQ5",
      title: "Coalesce concurrent token refreshes",
      status: "escalated",
      workflow_id: "bug",
      owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
      origin: "dispatched",
      urgency: "normal",
      atomic: false,
      model: "sonnet",
      created_at: "2026-08-31T09:00:00Z",
    },
    created_at: "2026-08-31T09:00:00Z",
    steps: [],
    acceptance_criteria: [],
    dependencies: [],
    stuck: {
      stopped_by: "blocked_by_policy",
      step_id: "implement",
      recourse: ["redirect_drone"],
      worktree_on_disk: true,
      drone_unheard: false,
      refused: [],
      refusals: 0,
      ...over,
    },
  };
}

describe("what a stopped job says it was refused", () => {
  it("draws nothing where nothing was refused", () => {
    expect(refusedIn(whole())).toBeUndefined();
  });

  it("draws nothing while the job's own read is in flight", () => {
    expect(refusedIn(null)).toBeUndefined();
  });

  it("draws nothing where fleet classified nothing at all", () => {
    // A Job Fleet serves no `stuck` for is not a Job that was refused nothing —
    // it is one nothing classified — and neither draws a list.
    const unclassified: JobWhole = { ...whole(), stuck: undefined };
    expect(refusedIn(unclassified)).toBeUndefined();
  });

  it("names the command, which is the whole of what was missing", () => {
    const drawn = refusedIn(whole({ refused: [refusal()], refusals: 1 }));
    expect(drawn?.refused[0]?.detail).toBe("cargo nextest run --package ipc 2>&1 | tail -80");
    expect(drawn?.refused[0]?.tool).toBe("Bash");
  });

  it("carries no reason where the harness gave none", () => {
    // `because` is empty on every `permission_denied` line observed, and an
    // empty string drawn as a reason is a blank line under every command.
    const drawn = refusedIn(whole({ refused: [refusal()], refusals: 1 }));
    expect(drawn?.refused[0]?.because).toBeUndefined();
  });

  it("carries the reason where it gave one", () => {
    const gave = refusal({ because: "rm is not on this drone's allowlist" });
    const drawn = refusedIn(whole({ refused: [gave], refusals: 1 }));
    expect(drawn?.refused[0]?.because).toBe("rm is not on this drone's allowlist");
  });

  it("keeps one row per refusal, never collapsing a repeat", () => {
    // A drone refused the same command five times is a drone that did not
    // learn, which is the most diagnostic thing on the screen.
    const again = [refusal(), refusal(), refusal(), refusal(), refusal()];
    expect(refusedIn(whole({ refused: again, refusals: 5 }))?.refused).toHaveLength(5);
  });

  it("says nothing about a cap where every refusal fits", () => {
    expect(refusedIn(whole({ refused: [refusal()], refusals: 1 }))?.note).toBeUndefined();
  });

  it("says the list is shorter than what happened where it is", () => {
    const drawn = refusedIn(whole({ refused: [refusal(), refusal()], refusals: 137 }));
    expect(drawn?.note).toBe("showing 2 of 137 refused calls");
  });

  it("groups the digits of a count a person has to read", () => {
    const drawn = refusedIn(whole({ refused: [refusal()], refusals: 12400 }));
    expect(drawn?.note).toContain("12,400");
  });

  it("never claims fewer refusals than there are rows", () => {
    // Fleet raises a total below what it kept rather than refusing the
    // classification, and this is the same guard from the reading side.
    const drawn = refusedIn(whole({ refused: [refusal(), refusal()], refusals: 1 }));
    expect(drawn?.note).toBeUndefined();
  });
});
