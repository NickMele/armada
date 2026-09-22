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

describe("how a member's work is linked to the one before it", () => {
  const child = (id: string, over = {}) =>
    sampleJob({ id, dispatched_by: parent.job.id, ...over });

  it("says nothing about the first, which has nothing before it", () => {
    const view = jobMembersOf(parent, [child("one"), child("two")]);

    expect(view.members[0]?.link).toBeUndefined();
  });

  it("is merged for every later member, the one link Fleet builds", () => {
    const view = jobMembersOf(parent, [child("one"), child("two"), child("three")]);

    expect(view.members.slice(1).map((member) => member.link)).toEqual(["merged", "merged"]);
  });

  it("does not read a pull request of its own as a link to the one before", () => {
    const view = jobMembersOf(parent, [child("one"), child("two", { landed: "merged" })], {
      two: { ...parent, delivery: { pull_request: "https://example.invalid/pull/1" } },
    });

    expect(view.members[1]?.link).toBe("merged");
    expect(view.members[1]?.landed).toBe(true);
  });
});

describe("whether a member landed", () => {
  const child = (over = {}) => sampleJob({ id: "child", dispatched_by: parent.job.id, ...over });

  it("is the merge on the Board row", () => {
    expect(jobMembersOf(parent, [child({ landed: "merged" })]).members[0]?.landed).toBe(true);
  });

  it("is the merge on the member's own delivery, where the row does not carry it", () => {
    const view = jobMembersOf(parent, [child()], {
      child: { ...parent, delivery: { landed: "merged" } },
    });

    expect(view.members[0]?.landed).toBe(true);
  });

  it("is false where the pull request closed without merging", () => {
    expect(jobMembersOf(parent, [child({ landed: "closed_unmerged" })]).members[0]?.landed).toBe(
      false,
    );
  });

  it("is false on a member that reached a successful status and has not merged", () => {
    expect(jobMembersOf(parent, [child({ status: "completed_success" })]).members[0]?.landed).toBe(
      false,
    );
  });

  it("is never dated, because the wire dates no merge", () => {
    const row = child({ landed: "merged", ended_at: "2026-09-22T11:00:00Z" });

    expect(jobMembersOf(parent, [row]).members[0]?.landed_at).toBeUndefined();
  });
});

describe("what the card draws beside the state", () => {
  const row = sampleJob({
    id: "child",
    dispatched_by: parent.job.id,
    branch: "armada/22-give-the-store-one-shape",
    tasks: { done: 2, working: 1, open: 0, dropped: 0 },
  });

  it("takes the branch and the task counts off the Board row alone", () => {
    const member = jobMembersOf(parent, [row]).members[0];

    expect(member?.branch).toBe("armada/22-give-the-store-one-shape");
    expect(member?.tasks?.done).toBe(2);
  });

  it("takes the pull request, the scope and the question off the member's own read", () => {
    const question = {
      step_id: "handoff",
      criterion_id: "c1",
      question: "Does the selector cover the empty store?",
      expected: "A test for the empty store",
      produced: "No test names it",
      consequence: "The empty store is unproven",
      asked_at: "2026-09-22T10:00:00Z",
    };
    const member = jobMembersOf(parent, [row], {
      child: {
        ...parent,
        write_targets: ["packages/settings/src/"],
        delivery: { pull_request: "https://example.invalid/pull/1591" },
        judge_question: question,
      },
    }).members[0];

    expect(member?.pull_request).toBe("https://example.invalid/pull/1591");
    expect(member?.scope).toEqual(["packages/settings/src/"]);
    expect(member?.question?.question).toBe("Does the selector cover the empty store?");
  });

  it("says nothing about a member nobody opened, rather than drawing an empty one", () => {
    const member = jobMembersOf(parent, [row]).members[0];

    expect(member?.pull_request).toBeUndefined();
    expect(member?.scope).toBeUndefined();
    expect(member?.question).toBeUndefined();
  });
});
