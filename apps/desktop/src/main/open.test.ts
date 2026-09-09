// Which of a Job's files main will let a click reach.
//
// **The set is the whole subject.** `open.ts` builds every path itself except
// the per-step records, which are keyed by things only Fleet holds and arrive
// as strings — so the property that keeps `shell.openPath` off an arbitrary
// path survives as membership: a string the open Job's detail does not name is
// refused before anything is joined to it.
//
// A record that *is* named answers `not_there` here, because nothing wrote the
// file. That is the assertion: `not_there` means the membership check passed
// and the filesystem was asked, and `not_named` means it never got that far.
//
// **Electron is mocked, and only `shell.openPath`**, for `forge.test.ts`'s
// reason: what is under test is the decision in front of the call.

import { beforeEach, describe, expect, it, vi } from "vitest";

const openPath = vi.fn(async (_path: string) => "");
vi.mock("electron", () => ({ shell: { openPath: (path: string) => openPath(path) } }));

const { openArtifact } = await import("./open");

import { NOTHING_YET } from "../shared/bridge";
import type { BridgeState } from "../shared/bridge";
import type { JobSummary, StepDetail } from "@armada/protocol";

const JOB_ID = "01M22TYSAE0023MADDP5ZQEYGW";
const MANIFEST = "01M1CNPKTV0018H2M1CXDNBK06";
const BRIEF = ".armada/briefs/12-the-job/regression_verify.1.gaming.assertion_weakened.txt";

function summary(): JobSummary {
  return {
    id: JOB_ID,
    handle: "12-the-job",
    title: "Give a finished Job's disk back without taking its record",
    status: "escalated",
    workflow_id: "bug",
    owner_manifest_id: MANIFEST,
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-09-09T09:00:00Z",
  };
}

/** The step that stopped on a gaming flag, with the brief the flag answers. */
function stopped(): StepDetail {
  return {
    step_id: "regression_verify",
    label: "Regression check",
    ordinal: 3,
    state: "stopped",
    check_runs: [],
    overridden: false,
    judged: [],
    flagged: [
      {
        pattern: "assertion_weakened",
        cited: "crates/store/src/retain.rs:41 — the sentence holds back less than it did",
        brief_path: BRIEF,
      },
    ],
    attempts: [],
    verdicts: [],
    entered_at: "2026-09-09T11:00:00Z",
    updated_at: "2026-09-09T11:10:00Z",
  };
}

/** Main holding that Job open, and the Manifest its paths are relative to. */
function holding(): BridgeState {
  return {
    ...NOTHING_YET,
    jobs: [summary()],
    holds: {
      ...NOTHING_YET.holds,
      manifests: [
        {
          id: MANIFEST,
          repository: "armada",
          path: "/Users/someone/code/armada/armada.yml",
          version: 1,
          checks: [],
        },
      ],
    },
    watched: {
      state: "read",
      jobId: JOB_ID,
      detail: {
        job: summary(),
        created_at: "2026-09-09T09:00:00Z",
        steps: [stopped()],
        acceptance_criteria: [],
        dependencies: [],
      },
    },
  };
}

beforeEach(() => openPath.mockClear());

describe("the records a stopped Job names", () => {
  it("lets a gaming flag's brief through to the filesystem", async () => {
    // The whole of #580's Bridge half: the flag draws a control, and without
    // this the press answers "not on the reading of this job Bridge is
    // holding" — a dead control on the one surface that explains the stop.
    await expect(openArtifact(holding(), JOB_ID, { kept: BRIEF, what: "brief" })).resolves.toEqual(
      { ok: false, why: "not_there", path: `/Users/someone/code/armada/${BRIEF}` },
    );
    expect(openPath).not.toHaveBeenCalled();
  });

  it("refuses a brief no step names, by name", async () => {
    const other = ".armada/briefs/12-the-job/implement.1.gaming.test_deleted.txt";
    await expect(
      openArtifact(holding(), JOB_ID, { kept: other, what: "brief" }),
    ).resolves.toEqual({
      ok: false,
      why: "not_named",
      path: `/Users/someone/code/armada/${other}`,
    });
    expect(openPath).not.toHaveBeenCalled();
  });
});
