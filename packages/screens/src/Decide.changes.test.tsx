// What should change at the gate: the listed changes go out with the note, and
// approving past them asks first. #907.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import type { JobSummary } from "@armada/protocol";
import { Decide } from "./Decide";
import { mount, unmount } from "./mounted";

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

type Sent = { approved: string[]; notes: string[] };

/** The gate with one small fix listed, and a record of what it sent. */
function listed(): Sent {
  const sent: Sent = { approved: [], notes: [] };
  mount(
    <Decide
      onNeedMaterial={() => {}}
      onNeedRemarks={() => {}}
      job={JOB}
      evidence={{ state: "none" }}
      remarks={{ state: "none" }}
      stale={false}
      deciding={false}
      onMerge={() => {}}
      onApprove={(jobId) => sent.approved.push(jobId)}
      onRequestChanges={(_, note) => sent.notes.push(note)}
      onReject={() => {}}
      onTakeUpRemarks={() => {}}
      onOpenRemarkLink={() => {}}
      changes={[{ id: "small-fix-0", from: "Small fix", text: "Move the migration into its own file." }]}
      onRemoveChange={() => {}}
    />,
  );
  return sent;
}

test("request changes sends the listed change with the typed words, as one note", async () => {
  const sent = listed();

  await userEvent.fill(
    page.getByRole("textbox", { name: "Anything else the drone should know" }),
    "Add a test that loads one.",
  );
  await userEvent.click(page.getByRole("button", { name: "Request changes" }));

  expect(sent.notes).toEqual([
    "What should change:\n- Small fix: Move the migration into its own file.\n\nAdd a test that loads one.",
  ]);
});

test("approving with a change still listed asks, and keeping them approves nothing", async () => {
  const sent = listed();

  await userEvent.click(page.getByRole("button", { name: "Approve the work" }));
  await expect.element(page.getByRole("dialog")).toBeVisible();
  expect(sent.approved, "the press approved past a listed change").toEqual([]);

  await userEvent.click(page.getByRole("dialog").getByRole("button", { name: "Keep them" }));
  expect(sent.approved).toEqual([]);
  await expect.element(page.getByRole("dialog")).not.toBeInTheDocument();
});

test("approving and dropping the listed changes takes the work once", async () => {
  const sent = listed();

  await userEvent.click(page.getByRole("button", { name: "Approve the work" }));
  await userEvent.click(
    page.getByRole("dialog").getByRole("button", { name: "Approve and drop them" }),
  );

  expect(sent.approved).toEqual([JOB.id]);
});
