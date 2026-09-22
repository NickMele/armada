// A parent Job and the Jobs landing under it, from `dispatched_by`.

import { describe, expect, it } from "vitest";

import { jobMembersOf } from "./members";
import { sampleDetail, sampleJob } from "./sample";

const parent = sampleDetail();

describe("which Jobs count as members", () => {
  it("is the rows whose dispatched_by names this Job, and no others", () => {
    const mine = sampleJob({ id: "child", dispatched_by: parent.job.id });
    const theirs = sampleJob({ id: "stranger", dispatched_by: "another-job" });
    const loose = sampleJob({ id: "loose" });

    const view = jobMembersOf(parent, [mine, theirs, loose]);

    expect(view.members.map((member) => member.job)).toEqual(["child"]);
  });

  it("is empty for an ordinary Job, which is a real answer and not a gap", () => {
    expect(jobMembersOf(parent, [sampleJob()]).members).toEqual([]);
  });

  it("names the parent by its own id and title", () => {
    const view = jobMembersOf(parent, []);

    expect(view.job).toBe(parent.job.id);
    expect(view.title).toBe(parent.job.title);
  });
});

describe("how a member's work is linked", () => {
  const child = (over = {}) =>
    sampleJob({ id: "child", dispatched_by: parent.job.id, ...over });

  it("is merged where the Board row says the pull request merged", () => {
    const view = jobMembersOf(parent, [child({ landed: "merged" })]);

    expect(view.members[0]?.link).toBe("merged");
  });

  it("is published where a delivery carries a pull request", () => {
    const view = jobMembersOf(parent, [child()], {
      child: { pull_request: "https://example.invalid/pull/1" },
    });

    expect(view.members[0]?.link).toBe("published");
  });

  it("is stacked where nothing was read, rather than guessing it was published", () => {
    expect(jobMembersOf(parent, [child()]).members[0]?.link).toBe("stacked");
  });

  it("reads a closed-unmerged pull request as published, not as merged", () => {
    const view = jobMembersOf(parent, [child({ landed: "closed_unmerged" })], {
      child: { pull_request: "https://example.invalid/pull/2" },
    });

    expect(view.members[0]?.link).toBe("published");
  });
});

describe("when a member landed", () => {
  it("is never said, because the wire dates no merge", () => {
    const child = sampleJob({
      id: "child",
      dispatched_by: parent.job.id,
      landed: "merged",
      ended_at: "2026-09-22T11:00:00Z",
    });

    expect(jobMembersOf(parent, [child]).members[0]?.landed_at).toBeUndefined();
  });
});
