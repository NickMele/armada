// Which acts the step header draws on a job that is still working.
//
// # Why this is a browser test and not a unit test
//
// `steering.test.ts` proves the reading; this proves the reading reaches a
// control. Those went out of step for a whole milestone — #145 made a redirect
// legal on a healthy drone, `keys.ts` already sent a person to job detail with
// `d` on exactly that row, and job detail drew nothing there — so the thing
// worth pinning is that a person watching a drone work can see the act, which
// is a question about the rendered tree and not about a function's return.
//
// The step's own acts are asserted here and the job's are not: the two kills
// are `Acts.tsx`'s, unchanged, and drawn from the same pointer they always were.

import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import type { JobSummary } from "@armada/protocol";
import { ACT_LABEL } from "./copy";
import { mount, unmount } from "./mounted";
import { StepActs } from "./StepActs";

afterEach(unmount);

/** A job that is running, with a drone on it. */
function job(over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: "01M130Y1380016YK5S0JXBXDQ5",
    title: "Coalesce concurrent token refreshes",
    status: "running",
    workflow_id: "bug",
    owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-08-31T09:00:00Z",
    branch: "armada/01M130Y1380016YK5S0JXBXDQ5",
    assigned_drone: "01M1D0X0000016YK5S0JXBXDQ5",
    ...over,
  };
}

/** The step header of a job that is working, with no detail read yet. */
function working(summary: JobSummary): void {
  mount(
    <StepActs
      job={summary}
      whole={null}
      render="working"
      acting={false}
      stale={false}
      onAct={() => {}}
      onRedirect={() => {}}
      onOverrule={() => {}}
      onRerun={() => {}}
    />,
  );
}

test("a person watching a drone work is offered the redirect", async () => {
  working(job());
  await expect
    .element(page.getByRole("button", { name: ACT_LABEL.redirect }))
    .toBeInTheDocument();
});

test("a working job with no drone on it is offered nothing", async () => {
  // There is no session for the instruction to go into, and nothing on the
  // wire holds a note for a running job — so the control is absent rather
  // than refused on the press.
  working(job({ assigned_drone: undefined }));
  expect(page.getByRole("button").elements()).toEqual([]);
});
