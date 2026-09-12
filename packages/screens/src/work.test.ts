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

import type { JobDetail, JobSummary, ManifestSummary, WorkflowSummary, Watched } from "@armada/protocol";
import type { OpenArtifact } from "./opening";
import { briefOf, stillReading, workOf, type WorkRehearsal } from "./work";

/** A Job with no server and nowhere the run sheet has opened. */
const noRehearsal: WorkRehearsal = {
  onRun: () => {},
  worktreeOnDisk: undefined,
  servers: [],
  onStopServer: () => {},
  onOpenServerLink: () => {},
};

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
  it("names every row before the job's own read answers", () => {
    // The Board's row carries the branch and the Drone, and the Manifest and
    // workflow holds are loaded for every job, so nothing here waits on the read.
    const withDrone: JobSummary = { ...job(), assigned_drone: "01M10B1V2A0011VRS6RA2SKPQ7" };
    const rows = workOf(noOpen, withDrone, null, manifest(), workflow(), noRehearsal);
    expect(rows.map((row) => row.iconLabel)).toEqual(["Worktree", "Branch", "Manifest", "Workflow", "Drone"]);
  });

  it("says the worktree is not written yet where the board's row has no branch", () => {
    const undispatched: JobSummary = { ...job() };
    delete undispatched.branch;
    const rows = workOf(noOpen, undispatched, null, manifest(), workflow(), noRehearsal);
    expect(rows.find((row) => row.iconLabel === "Worktree")?.meta).toBe("not written yet");
    expect(rows.map((row) => row.iconLabel)).not.toContain("Branch");
  });

  it("takes the branch from the job's own read once it answers", () => {
    const rows = workOf(noOpen, job(), detail({ branch: "fix/settings-split-selectors" }), manifest(), workflow(), noRehearsal);
    expect(rows.find((row) => row.iconLabel === "Branch")?.value).toBe("fix/settings-split-selectors");
  });

  it("falls back to the wire's own ids while the manifest and workflow holds are not there yet", () => {
    const rows = workOf(noOpen, job(), null, undefined, undefined, noRehearsal);
    expect(rows.find((row) => row.iconLabel === "Manifest")?.value).toBe(job().owner_manifest_id);
    expect(rows.find((row) => row.iconLabel === "Workflow")?.value).toBe(job().workflow_id);
  });
});

const JOB_ID = "01M130Y1380016YK5S0JXBXDQ5";

describe("whether this job's own read has answered yet", () => {
  it("is still reading before anything has been asked", () => {
    expect(stillReading({ state: "none" }, JOB_ID)).toBe(true);
  });

  it("is still reading while this job's own read is in flight", () => {
    expect(stillReading({ state: "reading", jobId: JOB_ID }, JOB_ID)).toBe(true);
  });

  it("is not still reading once this job's read has come back", () => {
    const read: Watched = { state: "read", jobId: JOB_ID, detail: detail() };
    expect(stillReading(read, JOB_ID)).toBe(false);
  });

  it("is not still reading once Fleet has answered that it will not answer", () => {
    const failed: Watched = { state: "failed", jobId: JOB_ID, outcome: { ok: false, why: "not_connected" } };
    expect(stillReading(failed, JOB_ID)).toBe(false);
  });

  it("is still reading for a job that is not the one just read", () => {
    // A stale reading from the job open before this one must not be mistaken
    // for this job's own answer — the whole reason `JobRead` carries a
    // `jobId` rather than one flag every per-job read shares.
    const read: Watched = { state: "read", jobId: "a-different-job", detail: detail() };
    expect(stillReading(read, JOB_ID)).toBe(true);
  });
});
