// The detail header's field run, and the two facts on it that are usually
// absent.
//
// **Most of this is about absence.** The pull request and what became of it
// appear on a handful of Jobs and on none of the rest, and the way a field run
// goes wrong is by drawing a slot that has nothing in it — so what is pinned
// here is which Jobs draw nothing, not just which draw something.
//
// The rest is about the moment the fact appears. A Job draws its pull request
// from the instant Fleet opens one, not from the instant somebody merges it:
// every Job that finishes is open and waiting on a reviewer, so a fact that
// waited for a merge would be missing exactly when it is wanted. `#422`.

import type { ReactNode } from "react";
import { describe, expect, it } from "vitest";

import { factsOf, pullRequestNumber } from "./facts";
import type { JobDetail, JobSummary } from "@armada/protocol";
import type { JobDetailField } from "@armada/components";

/** A row with everything the header reads. */
function job(over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: "01M130Y1380016YK5S0JXBXDQ5",
    handle: "12-a-job",
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

/** The run, as the header would build it. */
function run(whole: JobDetail | null): JobDetailField[] {
  return factsOf(job(), whole, Date.parse("2026-08-31T09:05:00Z"));
}

/** The labels the run drew, in order. */
function labels(whole: JobDetail | null): ReactNode[] {
  return run(whole).map((field) => field.label);
}

/** The one fact carrying an address, or `undefined`. */
function linked(whole: JobDetail | null): JobDetailField | undefined {
  return run(whole).find((field) => field.href !== undefined);
}

describe("the pull request it opened", () => {
  it("names an open pull request by its number, and links it", () => {
    const fact = linked(detail({ pull_request: "https://forge.invalid/org/repo/pull/4711" }));
    expect(fact?.label).toBe("Pull request");
    expect(fact?.value).toBe("#4711");
    expect(fact?.href).toBe("https://forge.invalid/org/repo/pull/4711");
  });

  it("never draws the address as the value", () => {
    const address = "https://forge.invalid/org/repo/pull/4711";
    // The whole of it stays reachable — the component puts it on `title` — but
    // a sixty-character value in a run of short readings is the thing the issue
    // said not to do.
    expect(linked(detail({ pull_request: address }))?.value).not.toBe(address);
  });

  it("draws it while nobody has merged it, which is every Job that just finished", () => {
    expect(linked(detail({ pull_request: "https://forge.invalid/pull/1" }))).toBeDefined();
  });

  it("draws nothing in a repository with no remote", () => {
    // Committed, nothing pushed, no pull request invented. Absent rather than
    // empty is the whole of what such a Job is owed here.
    expect(linked(detail({ commit: "abc123", pushed: "no remote" }))).toBeUndefined();
  });

  it("draws nothing on a Job whose detail has not arrived", () => {
    expect(linked(null)).toBeUndefined();
  });

  it("falls back to the words alone where the address carries no number", () => {
    // A forge that addresses a pull request by a slug. Still linked, and still
    // not the raw address.
    const fact = linked(detail({ pull_request: "https://forge.invalid/org/repo/pr/some-slug" }));
    expect(fact?.value).toBe("Pull request");
    expect(fact?.label).toBeUndefined();
  });

  it("sits beside the branch it was opened from", () => {
    const fields = run(detail({ pull_request: "https://forge.invalid/pull/4711" }));
    const branch = fields.findIndex((field) => field.value === job().branch);
    const address = fields.findIndex((field) => field.href !== undefined);
    expect(address).toBe(branch + 1);
  });
});

describe("what a number is read out of an address", () => {
  it("reads the forge's own numbering", () => {
    expect(pullRequestNumber("https://forge.invalid/org/repo/pull/4711")).toBe("#4711");
    expect(pullRequestNumber("https://forge.invalid/g/p/-/merge_requests/12")).toBe("#12");
  });

  it("takes the last run of digits, so a repository named in digits does not win", () => {
    expect(pullRequestNumber("https://forge.invalid/1234/9/pull/7")).toBe("#7");
  });

  it("ignores a query and a fragment", () => {
    expect(pullRequestNumber("https://forge.invalid/org/repo/pull/88?tab=files#3000")).toBe("#88");
  });

  it("answers null where there is no number to read", () => {
    expect(pullRequestNumber("https://forge.invalid/org/repo/pulls/open")).toBeNull();
  });
});

describe("did this land", () => {
  it("continues the pull request rather than standing beside it", () => {
    // One thought — what the branch came to — said to the depth the record can
    // say it. Two facts with the run's gap between them would read as two.
    const fields = run(
      detail({ pull_request: "https://forge.invalid/pull/1", landed: "merged" }),
    );
    const settled = fields.at(fields.findIndex((field) => field.href !== undefined) + 1);
    expect(settled).toEqual({ label: "merged", continues: true });
  });

  it("tells a pull request that was turned down from one that landed", () => {
    const drawn = labels(
      detail({ pull_request: "https://forge.invalid/pull/1", landed: "closed_unmerged" }),
    );
    expect(drawn).toContain("closed without merging");
    expect(drawn).not.toContain("merged");
  });

  it("draws nothing for a pull request nobody has merged yet", () => {
    // The state a pull request is in from the moment it exists. A word here
    // would be a slot on every open one saying that nothing has happened.
    const drawn = labels(detail({ pull_request: "https://forge.invalid/pull/1" }));
    expect(drawn).not.toContain("merged");
    expect(drawn).not.toContain("closed without merging");
  });

  it("opens a fact of its own where there is no address to continue", () => {
    // A Job old enough that Fleet recorded the verdict and not the address.
    // Sentence-initial, so it takes a capital.
    const drawn = labels(detail({ landed: "merged" }));
    expect(drawn).toContain("Merged");
  });

  it("draws nothing in a repository with no remote", () => {
    expect(labels(detail({ commit: "abc123", pushed: "no remote" }))).not.toContain("merged");
  });

  it("draws nothing on a Job whose detail has not arrived", () => {
    expect(labels(null)).not.toContain("merged");
  });
});

describe("Elapsed", () => {
  const now = Date.parse("2026-08-31T09:05:00Z");

  it("draws nothing for a Job that has never run", () => {
    const drawn = factsOf(job({ status: "awaiting_approval", started_at: undefined }), null, now);
    expect(drawn.find((field) => field.label === "Elapsed")).toBeUndefined();
  });

  it("draws the span from started_at once the Job has run", () => {
    const drawn = factsOf(job({ status: "running", started_at: "2026-08-31T09:00:00Z" }), null, now);
    expect(drawn.find((field) => field.label === "Elapsed")?.value).toBe("5m 00s");
  });
});

describe("repository", () => {
  it("leads the run, named by the Manifest id", () => {
    expect(run(null)[0]).toEqual({ value: "01M1CNPKTV0018H2M1CXDNBK06", mono: true });
  });
});

describe("who dispatched it", () => {
  const now = Date.now();
  // The fixture's own origin, `dispatched`, has no row — a baseline with no
  // dispatched-by fact to compare the rest against.
  const baseline = factsOf(job({ origin: "dispatched" }), null, now);

  it("closes the run with the registry's own sentence", () => {
    const drawn = factsOf(job({ origin: "auto_detected" }), null, now);
    expect(drawn.at(-1)).toEqual({ value: "Found by Fleet" });
  });

  it("draws nothing for sub_dispatched where an older Fleet sends no dispatched_by", () => {
    expect(factsOf(job({ origin: "sub_dispatched" }), null, now)).toEqual(baseline);
  });

  it("names the parent Job for sub_dispatched, since #1165", () => {
    const drawn = factsOf(
      job({ origin: "sub_dispatched", dispatched_by: "01PARENT0000000000000000" }),
      null,
      now,
    );
    expect(drawn.at(-1)).toEqual({
      value: "Sub-dispatched by 01PARENT0000000000000000",
    });
  });

  it("draws nothing for a wire origin the registry has no row for", () => {
    expect(factsOf(job({ origin: "unknown-origin" }), null, now)).toEqual(baseline);
  });

  // #1362. Where a job came from leads the sentence, and who pressed continues
  // it: one column answers two questions and neither reading may be dropped.
  it("says where from and who pressed, for a job dispatched off a Studio", () => {
    expect(factsOf(job({ origin: "studio_dispatched" }), null, now).at(-1)).toEqual({
      value: "From a Studio, by you",
    });
    expect(factsOf(job({ origin: "studio_helm_drafted" }), null, now).at(-1)).toEqual({
      value: "From a Studio, via Helm",
    });
  });
});

// #1362. The Studio that is still there is named under *Where things are*,
// where a value you reach lives. What the header carries is the other half:
// that the Studio is gone, which nothing else on the screen could say.
describe("a Studio that has been deleted", () => {
  const now = Date.now();
  const offAStudio = job({ origin: "studio_dispatched" });
  const gone = () => factsOf(offAStudio, detail(undefined), now);

  it("says so, once the read has answered with no Studio", () => {
    expect(gone().at(-1)).toEqual({ label: "That Studio has been deleted" });
  });

  it("says nothing while the Studio is still there — the row under Where things are has it", () => {
    const whole = {
      ...detail(undefined),
      from_studio: { studio_id: "01STUDIO", node_id: "01JOBNODE" },
    };
    expect(factsOf(offAStudio, whole, now).at(-1)).toEqual({
      value: "From a Studio, by you",
    });
  });

  it("claims nothing before the read lands", () => {
    expect(factsOf(offAStudio, null, now).at(-1)).toEqual({
      value: "From a Studio, by you",
    });
  });

  it("says nothing on a job that never came off a Studio", () => {
    expect(factsOf(job({ origin: "manual" }), detail(undefined), now).at(-1)).toEqual({
      value: "Dispatched by you",
    });
  });
});

// The quieter half of #1439: the job that took the work on says where it came
// from, in the slot a sub-dispatched job names its parent in.
describe("which job it replaced", () => {
  const now = Date.now();

  it("draws nothing on a job that replaced nothing", () => {
    expect(factsOf(job(), null, now).some((fact) => fact.label === "Redispatched from")).toBe(
      false,
    );
  });

  it("closes the run with the job it replaced", () => {
    const drawn = factsOf(job({ redispatched_from: "01FAILED000000000000000000" }), null, now);
    expect(drawn.at(-1)).toEqual({
      label: "Redispatched from",
      value: "01FAILED000000000000000000",
      mono: true,
    });
  });
});
