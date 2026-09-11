// The Board's arithmetic, case by case.
//
// **Every case here is a sentence `board.ts` already claims.** Its comments say
// which tab a status lands in and why the order of two tests matters; this is
// that document executed. Where a test's name reads as a claim about the
// product rather than about a function, that is deliberate — the claim is what
// is being pinned, and the function is how it is spelled today.
//
// The statuses are the wire's own, read through `JOB_LIFECYCLE`. Nothing here
// lists which are terminal: the registry says, and a test that retyped the list
// would go on passing after the registry changed.

import { describe, expect, it } from "vitest";

import {
  BOARD_TABS,
  DEFAULT_SORT,
  emptiedBy,
  FIRST_TAB,
  inTab,
  matches,
  sectionOf,
  sectionsOf,
  needsYou,
  sorted,
  tabOf,
  tabSuspended,
} from "./board";
import type { BoardTab } from "./board";

import type { JobSummary } from "@armada/protocol";
import type { WorkflowSummary } from "@armada/protocol";

/** A row with everything the Board reads, and nothing it does not. */
function job(over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: "01M130Y1380016YK5S0JXBXDQ5",
    handle: "12-a-job",
    title: "Coalesce concurrent token refreshes",
    status: "running",
    workflow_id: "bug",
    owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-08-31T09:00:00Z",
    ...over,
  };
}

describe("which tab a job is in", () => {
  it("puts every terminal status under Finished", () => {
    for (const status of ["completed_success", "completed_failed", "killed", "rejected", "superseded"]) {
      expect(tabOf(job({ status })), status).toBe("finished");
    }
  });

  it("puts a reclaimed job under Cleared, whichever terminal status it stopped at, and not under Finished", () => {
    // `#570`: a bulk clear used to delete the row it was asked to keep. The
    // fix is a row fact, not a registry one — `reclaimed_at` is not on any
    // lifecycle row, so this is the one rule in the function that is not read
    // off `JOB_LIFECYCLE`.
    for (const status of ["completed_success", "killed", "rejected"]) {
      expect(
        tabOf(job({ status, reclaimed_at: "2026-09-09T00:00:00.000Z" })),
        status,
      ).toBe("cleared");
    }
  });

  it("puts a job somebody has taken over under Running, not Needs you", () => {
    // `piloted` is `Working` and its actor is a `Person`. Asking the mode first
    // is the whole of why a piloted job does not read as one waiting on you.
    expect(tabOf(job({ status: "piloted" }))).toBe("running");
  });

  it("puts the gates a person answers under Needs you", () => {
    for (const status of ["awaiting_approval", "awaiting_attestation", "awaiting_review", "escalated"]) {
      expect(tabOf(job({ status })), status).toBe("needs-you");
    }
  });

  it("sorted a newly minted status without a line being written here", () => {
    // `awaiting_repair` landed in `job-statuses.toml` carrying `Waited on` and
    // `Person`, and the tab followed from the registry rather than from a list
    // — which is the property the four positive rules exist for. The Board's
    // filter cost nothing but the codegen run, and this case is what says so.
    expect(tabOf(job({ status: "awaiting_repair" }))).toBe("needs-you");
  });

  it("puts a queued job under Queued", () => {
    expect(tabOf(job({ status: "queued" }))).toBe("queued");
  });

  it("lifts a running job whose drone is asking into Needs you", () => {
    // The rule that is not a lifecycle row. Without it a question sits under
    // Running and nobody sees it until they open that job.
    expect(tabOf(job({ status: "running" }))).toBe("running");
    expect(tabOf(job({ status: "running", asking: true }))).toBe("needs-you");
  });

  it("answers null for a status this build has never heard of", () => {
    // Not a fifth tab, and not silently Queued. The counts stop summing, which
    // is what makes a registry change visible instead of invisible.
    expect(tabOf(job({ status: "translated_into_greek" }))).toBeNull();
  });

  it("counts the unplaceable under All and under no state tab", () => {
    const stranger = job({ status: "translated_into_greek" });
    expect(inTab(stranger, "all")).toBe(true);
    for (const tab of ["needs-you", "running", "queued", "finished", "cleared"] as const) {
      expect(inTab(stranger, tab), tab).toBe(false);
    }
  });

  it("reads needsYou off the same rule the tab uses", () => {
    expect(needsYou(job({ status: "awaiting_review" }))).toBe(true);
    expect(needsYou(job({ status: "running" }))).toBe(false);
    expect(needsYou(job({ status: "running", asking: true }))).toBe(true);
  });
});

describe("the tab strip", () => {
  it("keys the tabs by their position, so a tab moving moves its key", () => {
    expect(BOARD_TABS.map((tab) => tab.shortcut)).toEqual(["1", "2", "3", "4", "5", "6"]);
    expect(BOARD_TABS.map((tab) => tab.id)).toEqual([
      "all",
      "needs-you",
      "running",
      "queued",
      "finished",
      "cleared",
    ]);
  });

  it("opens on All, and sorts critical first", () => {
    expect(FIRST_TAB).toBe("all");
    expect(DEFAULT_SORT).toBe("critical_first");
  });
});

describe("the search", () => {
  const workflows: readonly WorkflowSummary[] = [
    { id: "bug", name: "bug", version: 1, steps: [], manifest_id: "armada" },
  ];

  it("suspends the tab while the field holds text, and not while it holds space", () => {
    expect(tabSuspended("")).toBe(false);
    expect(tabSuspended("   ")).toBe(false);
    expect(tabSuspended("auth")).toBe(true);
  });

  it("matches every job when nothing is typed", () => {
    expect(matches(job(), "", workflows)).toBe(true);
    expect(matches(job(), "  ", workflows)).toBe(true);
  });

  it("matches on the fields the row actually shows", () => {
    const row = job({
      title: "Coalesce concurrent token refreshes",
      id: "01M130Y1380016YK5S0JXBXDQ5",
      handle: "12-a-job",
      branch: "armada/refresh-coalescing",
      current_step_id: "implement",
      workflow_id: "bug",
    });
    for (const needle of ["token", "01M130Y", "refresh-coalescing", "implement", "bug"]) {
      expect(matches(row, needle, workflows), needle).toBe(true);
    }
  });

  it("matches the workflow by the name a person reads, not only by its id", () => {
    const named: readonly WorkflowSummary[] = [
      { id: "wf_01", name: "Fix a bug", version: 1, steps: [], manifest_id: "armada" },
    ];
    expect(matches(job({ workflow_id: "wf_01" }), "Fix a bug", named)).toBe(true);
  });

  it("ignores case and surrounding space, because a person types neither carefully", () => {
    expect(matches(job({ title: "Coalesce" }), "  COALESCE ", workflows)).toBe(true);
  });

  it("does not match a field the row does not carry", () => {
    expect(matches(job({ branch: undefined }), "armada/", workflows)).toBe(false);
    expect(matches(job(), "sonnet", workflows)).toBe(false);
  });
});

describe("the order", () => {
  const early = job({ id: "a", status: "running", created_at: "2026-08-30T09:00:00Z" });
  const late = job({ id: "b", status: "running", created_at: "2026-08-31T09:00:00Z" });
  const waiting = job({ id: "c", status: "awaiting_review", created_at: "2026-08-31T12:00:00Z" });

  it("lifts the needs-you cluster, then goes oldest first inside every group", () => {
    expect(sorted([late, early, waiting], "critical_first").map((row) => row.id)).toEqual([
      "c",
      "a",
      "b",
    ]);
  });

  it("lifts nothing under Oldest first, and orders the same jobs by age alone", () => {
    expect(sorted([late, waiting, early], "oldest_first").map((row) => row.id)).toEqual([
      "a",
      "b",
      "c",
    ]);
  });

  it("sorts a job whose date will not parse last, never first", () => {
    const corrupt = job({ id: "z", status: "running", created_at: "not a date" });
    expect(sorted([corrupt, late, early], "oldest_first").map((row) => row.id)).toEqual([
      "a",
      "b",
      "z",
    ]);
  });

  it("breaks a tie on the id, so the order does not shuffle between renders", () => {
    const first = job({ id: "aaa", status: "running", created_at: "2026-08-31T09:00:00Z" });
    const second = job({ id: "bbb", status: "running", created_at: "2026-08-31T09:00:00Z" });
    expect(sorted([second, first], "oldest_first").map((row) => row.id)).toEqual(["aaa", "bbb"]);
  });

  it("does not reorder the caller's array", () => {
    const held = [late, early];
    sorted(held, "oldest_first");
    expect(held.map((row) => row.id)).toEqual(["b", "a"]);
  });
});

describe("what emptied the list", () => {
  it("names the search, not the tab, while a search is running", () => {
    expect(emptiedBy("running", "auth")).toBe("No jobs match your search.");
  });

  it("says nothing at all when no filter is set — that is a Manifest with no jobs", () => {
    expect(emptiedBy("all", "")).toBeNull();
  });

  it("names the tab that did it, in the words that tab already uses", () => {
    const said: [BoardTab, string][] = [
      ["needs-you", "Nothing needs you."],
      ["running", "Nothing is running."],
      ["queued", "Nothing is queued."],
      ["finished", "Nothing has finished."],
      ["cleared", "Nothing has been cleared."],
    ];
    for (const [tab, sentence] of said) {
      expect(emptiedBy(tab, ""), tab).toBe(sentence);
    }
  });
});

describe("the All tab's sections", () => {
  it("draws what needs you, what runs, what waits and what is over, in that order", () => {
    const rows = [
      job({ id: "a", status: "completed_success" }),
      job({ id: "b", status: "queued" }),
      job({ id: "c", status: "running" }),
      job({ id: "d", status: "awaiting_approval" }),
    ];
    expect(sectionsOf(rows).map((section) => section.id)).toEqual([
      "needs-you",
      "running",
      "queued",
      "done",
    ]);
  });

  it("puts a cleared job under Done with the finished ones", () => {
    expect(sectionOf(job({ status: "completed_success", reclaimed_at: "2026-09-01T00:00:00Z" }))).toBe(
      "done",
    );
  });

  it("draws no section with nothing in it", () => {
    expect(sectionsOf([job({ status: "running" })]).map((section) => section.id)).toEqual(["running"]);
  });

  it("keeps the order each section was handed", () => {
    const rows = [job({ id: "second", status: "running" }), job({ id: "first", status: "running" })];
    expect(sectionsOf(rows)[0]?.jobs.map((row) => row.id)).toEqual(["second", "first"]);
  });

  it("keeps a job no tab claims, under a label of its own", () => {
    expect(sectionOf(job({ status: "not_a_status_the_registry_has" }))).toBe("other");
  });
});
