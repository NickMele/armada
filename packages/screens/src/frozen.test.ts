// What a terminal Job does to the step beneath it.
//
// **The rule is a rendering rule and it has to stay one.** The wire still says
// `running` under a failed Job, and every case below asserts that nothing here
// invents a step state Fleet does not have: it either overrides a live claim or
// it answers nothing at all.

import { describe, expect, it } from "vitest";

import { activityFor, frozenBeneath } from "./frozen";

describe("the step activity a job status implies", () => {
  it("borrows the step glyph that means the same thing one level down", () => {
    expect(activityFor("running")).toBe("running");
    expect(activityFor("completed_failed")).toBe("failed");
    expect(activityFor("completed_success")).toBe("advanced");
    expect(activityFor("killed")).toBe("killed");
  });

  it("borrows nothing for a status with no step counterpart", () => {
    // `rejected` and `superseded` have no step state in any roster. Claiming
    // one would draw a mark that means something else.
    expect(activityFor("rejected")).toBeUndefined();
    expect(activityFor("superseded")).toBeUndefined();
    expect(activityFor("awaiting_review")).toBeUndefined();
  });
});

describe("a step beneath a job that is over", () => {
  it("overrides a step still claiming to be live", () => {
    for (const state of ["running", "retrying", "awaiting_human"]) {
      expect(frozenBeneath("completed_failed", state), state).toEqual({
        activity: "failed",
        word: "failed",
      });
    }
  });

  it("leaves a step whose own state is already settled alone", () => {
    // `advanced` and `stopped` are settled, and `not_started` is honest: a step
    // the job never reached did not fail, whatever ended the job above it.
    for (const state of ["not_started", "advanced", "stopped"]) {
      expect(frozenBeneath("completed_failed", state), state).toBeUndefined();
    }
  });

  it("does nothing at all while the job is still going", () => {
    for (const status of ["running", "queued", "piloted", "awaiting_review", "escalated"]) {
      expect(frozenBeneath(status, "running"), status).toBeUndefined();
    }
  });

  it("takes the word from the job's own verb", () => {
    expect(frozenBeneath("killed", "running")?.word).toBe("killed");
    expect(frozenBeneath("completed_success", "running")?.word).toBe("done");
  });

  it("falls to not_started where a terminal status has no step counterpart", () => {
    // Wrong about the step and right about what is known — and it still stops
    // the pulse, the hue and the clock, which is the whole of what a terminal
    // job owes the step beneath it.
    expect(frozenBeneath("rejected", "running")).toEqual({
      activity: "not_started",
      word: "rejected",
    });
    expect(frozenBeneath("superseded", "awaiting_human")).toEqual({
      activity: "not_started",
      word: "superseded",
    });
  });

  it("says nothing about a status this build has never heard of", () => {
    // Terminality is read from the registry, never listed here, so an unknown
    // status is not terminal and freezes nothing.
    expect(frozenBeneath("translated_into_greek", "running")).toBeUndefined();
  });
});
