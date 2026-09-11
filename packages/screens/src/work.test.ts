// What the brief says a job is holding, and what it says when it is holding
// nothing.
//
// **The waiting note is the half worth a test.** The other two are the wire's
// own strings passed through, and this one has a lifetime: fleet clears it the
// instant a drone's opening brief is built from it, so the block has to be gone
// on the same read that empties the field. A badge that outlived the note would
// tell somebody an instruction is still coming after it has been delivered,
// which is worse than not drawing it at all.

import { describe, expect, it } from "vitest";

import type { JobDetail, JobSummary, ManifestSummary, WorkflowSummary } from "@armada/protocol";
import type { OpenArtifact } from "./opening";
import { briefOf, workOf } from "./work";

function job(): JobSummary {
  return {
    id: "01M130Y1380016YK5S0JXBXDQ5",
    handle: "12-a-job",
    title: "Coalesce concurrent token refreshes",
    status: "escalated",
    workflow_id: "bug",
    owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-08-31T09:00:00Z",
    branch: "armada/01M130Y1380016YK5S0JXBXDQ5",
  };
}

function detail(over: Partial<JobDetail> = {}): JobDetail {
  return {
    job: job(),
    created_at: "2026-08-31T09:00:00Z",
    steps: [],
    acceptance_criteria: [],
    dependencies: [],
    facts: "The refresh path is in `auth/session.ts`.",
    ...over,
  };
}

describe("what the brief says is waiting", () => {
  it("draws a note nobody has opened with, in the words it was written in", () => {
    // Two acts leave one — sending work back at a gate, and restarting a step
    // with something to say — and neither is named here. What the field says is
    // that an instruction is on the record and no drone has it yet, which is
    // one fact whichever act wrote it.
    const brief = briefOf(
      detail({ redirect_waiting: { note: "Delete that test, it tests the old behaviour." } }),
    );
    expect(brief.waiting).toBe("Delete that test, it tests the old behaviour.");
  });

  it("draws nothing on a job nobody has typed into", () => {
    // The ordinary state of nearly every job ever drawn. `JobBrief` has no
    // `waitingAbsent` for that reason, and this is the half that has to answer
    // `undefined` for it to stay true.
    expect(briefOf(detail()).waiting).toBeUndefined();
  });

  it("draws nothing once the note has been delivered", () => {
    // The record clears the field on the spawn that opens with it, so absent is
    // both "nobody wrote one" and "the one somebody wrote has gone in". Neither
    // is a thing to draw, which is why one absence answers both.
    const delivered = detail({ redirect_waiting: undefined });
    expect(briefOf(delivered).waiting).toBeUndefined();
  });

  it("keeps the note out of the facts it sits above", () => {
    // Two strings on one panel, and only one of them is going somewhere. A note
    // folded into `facts` would read as context the job was dispatched with.
    const brief = briefOf(detail({ redirect_waiting: { note: "Start from the failing case." } }));
    expect(brief.facts).toBe("The refresh path is in `auth/session.ts`.");
    expect(brief.waiting).toBe("Start from the failing case.");
  });
});

const noOpen: OpenArtifact = () => Promise.resolve({ ok: true });

function manifest(): ManifestSummary {
  return {
    id: "01M1CNPKTV0018H2M1CXDNBK06",
    repository: "armada",
    path: "/repo/armada.yml",
    records_root: "/repo/.armada",
    version: 1,
    checks: [],
  };
}

function workflow(): WorkflowSummary {
  return { id: "bug", name: "bug", version: 1, steps: [], manifest_id: "01M1CNPKTV0018H2M1CXDNBK06" };
}

describe("where the work is", () => {
  it("names the worktree, the manifest, the workflow and the drone before the job's own read answers", () => {
    // Four of five rows come off `JobSummary` and the holds already loaded
    // for every job on the board — none of them need this job's own detail
    // read, so they draw before it lands rather than waiting beside Branch,
    // the one row that does.
    const rows = workOf(noOpen, job(), null, manifest(), workflow());
    const labels = rows.map((row) => row.iconLabel);
    expect(labels).toEqual(["Worktree", "Manifest", "Workflow"]);
  });

  it("names the drone once the job has one, still with no read in hand", () => {
    const withDrone: JobSummary = { ...job(), assigned_drone: "01M10B1V2A0011VRS6RA2SKPQ7" };
    const rows = workOf(noOpen, withDrone, null, manifest(), workflow());
    expect(rows.map((row) => row.iconLabel)).toContain("Drone");
  });

  it("says nothing about whether the worktree is written yet until the read answers", () => {
    const rows = workOf(noOpen, job(), null, manifest(), workflow());
    const worktree = rows.find((row) => row.iconLabel === "Worktree");
    expect(worktree?.meta).toBeUndefined();
  });

  it("adds the branch once the job's own read comes back", () => {
    const rows = workOf(noOpen, job(), detail({ branch: "fix/settings-split-selectors" }), manifest(), workflow());
    const branch = rows.find((row) => row.iconLabel === "Branch");
    expect(branch?.value).toBe("fix/settings-split-selectors");
  });

  it("falls back to the wire's own ids while the manifest and workflow holds are not there yet", () => {
    const rows = workOf(noOpen, job(), null, undefined, undefined);
    expect(rows.find((row) => row.iconLabel === "Manifest")?.value).toBe(job().owner_manifest_id);
    expect(rows.find((row) => row.iconLabel === "Workflow")?.value).toBe(job().workflow_id);
  });
});
