// The detail header's field run, and the one fact on it that is usually absent.
//
// **Every case is about absence.** The merge fact appears on a handful of Jobs
// and on none of the rest, and the way a field run goes wrong is by drawing a
// slot that has nothing in it — so what is pinned here is which Jobs draw
// nothing, not just which draw something.

import type { ReactNode } from "react";
import { describe, expect, it } from "vitest";

import { factsOf } from "./facts";
import type { JobDetail, JobSummary } from "@armada/protocol";

/** A row with everything the header reads. */
function job(over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: "01M130Y1380016YK5S0JXBXDQ5",
    title: "Coalesce concurrent token refreshes",
    status: "completed_success",
    workflow_id: "bug",
    owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-08-31T09:00:00Z",
    branch: "armada/01M130Y1380016YK5S0JXBXDQ5",
    ...over,
  };
}

/** A finished Job's detail, with whatever its branch came to. */
function detail(delivery: JobDetail["delivery"]): JobDetail {
  return {
    job: job(),
    created_at: "2026-08-31T09:00:00Z",
    steps: [],
    acceptance_criteria: [],
    dependencies: [],
    ...(delivery === undefined ? {} : { delivery }),
  };
}

/** The labels the run drew, in order. */
function labels(whole: JobDetail | null): ReactNode[] {
  return factsOf(job(), whole, undefined, Date.parse("2026-08-31T09:05:00Z")).map(
    (field) => field.label,
  );
}

describe("did this land", () => {
  it("says a pull request merged", () => {
    const drawn = labels(
      detail({ pull_request: "https://forge.invalid/pull/1", landed: "merged" }),
    );
    expect(drawn).toContain("Merged");
  });

  it("tells a pull request that was turned down from one that landed", () => {
    const drawn = labels(
      detail({ pull_request: "https://forge.invalid/pull/1", landed: "closed_unmerged" }),
    );
    expect(drawn).toContain("Closed, not merged");
    expect(drawn).not.toContain("Merged");
  });

  it("draws nothing for a pull request nobody has merged yet", () => {
    // The state a pull request is in from the moment it exists. A label here
    // would be a slot on every open one saying that nothing has happened.
    const drawn = labels(detail({ pull_request: "https://forge.invalid/pull/1" }));
    expect(drawn).not.toContain("Merged");
    expect(drawn).not.toContain("Closed, not merged");
  });

  it("draws nothing in a repository with no remote", () => {
    // Committed, nothing pushed, no pull request invented. Absent rather than
    // empty is the whole of what such a Job is owed here.
    const drawn = labels(detail({ commit: "abc123", pushed: "no remote" }));
    expect(drawn).not.toContain("Merged");
  });

  it("draws nothing on a Job whose detail has not arrived", () => {
    expect(labels(null)).not.toContain("Merged");
  });
});
