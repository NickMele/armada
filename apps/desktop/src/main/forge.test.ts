// What main will and will not hand to the OS when a person clicks a Job's pull
// request.
//
// **The address is the whole subject.** Everything else in this module is a
// lookup; the thing worth pinning is that the string reaching `openExternal` is
// the one Fleet recorded and never one the renderer composed, and that a record
// carrying something other than a web address is refused by name rather than
// opened. That is the difference between a link and an arbitrary URL handler,
// and it is one comparison — which is exactly the kind of line that gets
// deleted as redundant by somebody who did not know why it was there.
//
// **Electron is mocked, and only `shell.openExternal`.** The runner is node
// with nothing from Electron by design; what is asserted here is the decision
// in front of the call, so the call is a spy and the shell is not under test.

import { beforeEach, describe, expect, it, vi } from "vitest";

const openExternal = vi.fn(async (_url: string) => undefined);
vi.mock("electron", () => ({ shell: { openExternal: (url: string) => openExternal(url) } }));

const { openPullRequest } = await import("./forge");

import { NOTHING_YET } from "../shared/bridge";
import type { BridgeState } from "../shared/bridge";
import type { JobDetail, JobSummary } from "@armada/protocol";

const JOB_ID = "01M1N1TJB3002E49K150S7AF2B";
const ADDRESS = "https://forge.invalid/NickMele/armada/pull/4711";

function summary(): JobSummary {
  return {
    id: JOB_ID,
    title: "Coalesce concurrent token refreshes",
    status: "completed_success",
    workflow_id: "bug",
    owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-09-02T09:00:00Z",
  };
}

/** Main holding one Job open, with whatever its branch came to. */
function holding(delivery: JobDetail["delivery"]): BridgeState {
  return {
    ...NOTHING_YET,
    jobs: [summary()],
    watched: {
      state: "read",
      jobId: JOB_ID,
      detail: {
        job: summary(),
        created_at: "2026-09-02T09:00:00Z",
        steps: [],
        acceptance_criteria: [],
        dependencies: [],
        ...(delivery === undefined ? {} : { delivery }),
      },
    },
  };
}

beforeEach(() => openExternal.mockClear());

describe("opening a job's pull request", () => {
  it("hands over the address Fleet recorded, and nothing else", async () => {
    await expect(openPullRequest(holding({ pull_request: ADDRESS }), JOB_ID)).resolves.toEqual({
      ok: true,
    });
    expect(openExternal).toHaveBeenCalledWith(ADDRESS);
  });

  it("refuses an address that is not a web address, by name", async () => {
    // Nothing a person can bring about, and it is still refused rather than
    // opened: `shell.openExternal` will hand a registered protocol handler
    // whatever it is given, so the scheme is checked on a value that already
    // came from Fleet.
    const state = holding({ pull_request: "file:///Users/user/.ssh/id_ed25519" });
    await expect(openPullRequest(state, JOB_ID)).resolves.toEqual({
      ok: false,
      why: "not_addressable",
      address: "file:///Users/user/.ssh/id_ed25519",
    });
    expect(openExternal).not.toHaveBeenCalled();
  });

  it("refuses a string that is not an address at all", async () => {
    const state = holding({ pull_request: "armada/01M1N1TJB3002E49K150S7AF2B" });
    await expect(openPullRequest(state, JOB_ID)).resolves.toMatchObject({
      why: "not_addressable",
    });
    expect(openExternal).not.toHaveBeenCalled();
  });

  it("says so where the reading it is holding names no pull request", async () => {
    // The race the screen cannot draw: the Job was re-read between the draw
    // and the click. A silent refusal here is the dead click `#422` is about.
    await expect(
      openPullRequest(holding({ commit: "9f2c1ab", pushed: "no remote" }), JOB_ID),
    ).resolves.toEqual({ ok: false, why: "no_address" });
    expect(openExternal).not.toHaveBeenCalled();
  });

  it("says so where it no longer holds the job", async () => {
    await expect(openPullRequest(NOTHING_YET, JOB_ID)).resolves.toEqual({
      ok: false,
      why: "unknown_job",
    });
  });

  it("reads the address off the job that is open, never off another one", async () => {
    // `watched` is one Job. A click arriving for a different id gets that id's
    // answer — no address — rather than the open Job's address under the wrong
    // name.
    const state = holding({ pull_request: ADDRESS });
    state.jobs = [summary(), { ...summary(), id: "01M1OTHER0000000000000000" }];
    await expect(openPullRequest(state, "01M1OTHER0000000000000000")).resolves.toEqual({
      ok: false,
      why: "no_address",
    });
    expect(openExternal).not.toHaveBeenCalled();
  });

  it("carries the machine's own words where it declines", async () => {
    openExternal.mockRejectedValueOnce(new Error("no application knows how to open this"));
    await expect(openPullRequest(holding({ pull_request: ADDRESS }), JOB_ID)).resolves.toMatchObject(
      { why: "refused", address: ADDRESS },
    );
  });
});
