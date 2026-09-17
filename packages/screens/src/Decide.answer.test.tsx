// Fleet's answer at the review gate lands on the answer pressed, and on none of
// the other three. `data-answer` is read because no accessible property carries
// the line along a button's edge.

import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import type { JobSummary } from "@armada/protocol";
import { Decide } from "./Decide";
import { mount, rerender, unmount } from "./mounted";
import type { ActAnswer, DecidingAct } from "./pending";

afterEach(unmount);

const JOB: JobSummary = {
  id: "01M130Y1380016YK5S0JXBXDQ5",
  handle: "12-a-job",
  title: "Coalesce concurrent token refreshes",
  status: "awaiting_review",
  workflow_id: "bug",
  owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
  origin: "dispatched",
  urgency: "normal",
  atomic: false,
  model: "sonnet",
  created_at: "2026-08-31T09:00:00Z",
  branch: "armada/01M130Y1380016YK5S0JXBXDQ5",
};

/** The gate with a pull request, over one answer and whatever is out. */
function gate(answered: ActAnswer | undefined, decidingAct?: DecidingAct, put: typeof mount = mount): void {
  put(
    <Decide
      onNeedMaterial={() => {}}
      onNeedRemarks={() => {}}
      job={JOB}
      evidence={{ state: "none" }}
      remarks={{ state: "none" }}
      stale={false}
      deciding={decidingAct !== undefined}
      decidingAct={decidingAct}
      answered={answered}
      pullRequest="https://git.example/armada/armada/pull/533"
      onMerge={() => {}}
      onApprove={() => {}}
      onRequestChanges={() => {}}
      onReject={() => {}}
      onTakeUpRemarks={() => {}}
      onOpenRemarkLink={() => {}}
    />,
  );
}

test("a refused merge answers on merge alone", async () => {
  gate({ act: "merge", answer: "refused" });
  await expect
    .element(page.getByRole("button", { name: "Merge and take the work" }))
    .toHaveAttribute("data-answer", "refused");
  await expect
    .element(page.getByRole("button", { name: "Approve the work" }))
    .not.toHaveAttribute("data-answer");
});

test("an accepted approve answers accepted, and the next press clears it", async () => {
  gate({ act: "approve", answer: "accepted" });
  await expect
    .element(page.getByRole("button", { name: "Approve the work" }))
    .toHaveAttribute("data-answer", "accepted");
  gate(undefined, "approve", rerender);
  await expect
    .element(page.getByRole("button", { name: "Approving…" }))
    .not.toHaveAttribute("data-answer");
});

test("an answer to an act away from the four marks none of them", async () => {
  gate({ act: "take_up_remarks", answer: "refused" });
  for (const name of ["Merge and take the work", "Approve the work", "Reject the work"]) {
    await expect.element(page.getByRole("button", { name })).not.toHaveAttribute("data-answer");
  }
});
