// Merging asks first, and the other three answers still do not.
//
// # Why this is a browser test and not a story
//
// The dialog is wired here, in a screen. `xtask/src/rules_layers.rs` puts
// `@armada/components` below `@armada/screens` and refuses the import the other
// way, so no story can mount `Decide` — a story on `Primitives/Dialog` proves
// what the layer does with focus and `Enter`, and this proves this screen puts
// a merge behind it.
//
// # What is actually at risk
//
// Merging is the one act in Armada that writes into a repository Fleet did not
// make, and nothing in Bridge takes it back. The failure to catch is not a
// double press — `deciding` blocks that and always did — but a first press that
// lands the work without anybody meaning it. So every assertion here is about
// what did **not** happen: `onMerge` uncalled after the press, uncalled after
// `Enter`, uncalled after `Esc`, and called exactly once after the deliberate
// move to the other control.
//
// The three answers beside it are asserted for the same reason in reverse. A
// change that put every answer behind a dialog would pass every test above and
// be the gate in the wrong place, which is the thing the issue that asked for
// this one warned against.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import type { Diff, Evidence, JobSummary } from "@armada/protocol";
import { Decide } from "./Decide";
import { mount, unmount } from "./mounted";

afterEach(unmount);

/** A job holding an open pull request, waiting for somebody at the gate. */
const JOB: JobSummary = {
  id: "01M130Y1380016YK5S0JXBXDQ5",
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

/** Nothing read yet. The decision does not depend on either read arriving. */
const NO_EVIDENCE: Evidence = { state: "none" };
const NO_DIFF: Diff = { state: "none" };

/** What each answer was told, in the order it was told. */
type Sent = { merged: string[]; approved: string[]; changes: string[]; rejected: string[] };

/** The gate, with a pull request to merge, and a record of what it sent. */
function gate(): Sent {
  const sent: Sent = { merged: [], approved: [], changes: [], rejected: [] };
  mount(
    <Decide
      onNeedMaterial={() => {}}
      job={JOB}
      evidence={NO_EVIDENCE}
      diff={NO_DIFF}
      stale={false}
      deciding={false}
      pullRequest="https://forge.example/armada/pull/533"
      onMerge={(jobId) => sent.merged.push(jobId)}
      onApprove={(jobId) => sent.approved.push(jobId)}
      onRequestChanges={(jobId) => sent.changes.push(jobId)}
      onReject={(jobId) => sent.rejected.push(jobId)}
    />,
  );
  return sent;
}

/**
 * The confirm control, scoped to the dialog.
 *
 * **The opener and the confirm carry the same words on purpose** — the design
 * contract says an act keeps its name through the flow — so a query by name
 * alone finds two buttons.
 */
function confirmMerge() {
  return page.getByRole("dialog").getByRole("button", { name: "Merge and take the work" });
}

test("pressing merge asks rather than merging", async () => {
  const sent = gate();

  await userEvent.click(page.getByRole("button", { name: "Merge and take the work" }));

  await expect.element(page.getByRole("dialog")).toBeVisible();
  expect(sent.merged, "the press merged").toEqual([]);
});

test("Enter cancels the merge, because Cancel is what holds focus", async () => {
  const sent = gate();
  await userEvent.click(page.getByRole("button", { name: "Merge and take the work" }));

  // The contract's rule, run rather than described: `Enter` fires whatever
  // holds focus, and on a plain confirmation that is Cancel.
  const cancel = page.getByRole("dialog").getByRole("button", { name: "Cancel" });
  await expect.element(cancel).toHaveFocus();

  await userEvent.keyboard("{Enter}");

  expect(sent.merged, "Enter merged past the focused Cancel").toEqual([]);
  await expect.element(page.getByRole("dialog")).not.toBeInTheDocument();
});

test("Esc cancels the merge too", async () => {
  const sent = gate();
  await userEvent.click(page.getByRole("button", { name: "Merge and take the work" }));

  await userEvent.keyboard("{Escape}");

  expect(sent.merged, "Esc merged").toEqual([]);
  await expect.element(page.getByRole("dialog")).not.toBeInTheDocument();
});

test("confirming merges once, and names the job it was asked about", async () => {
  const sent = gate();
  await userEvent.click(page.getByRole("button", { name: "Merge and take the work" }));

  // **The half that keeps the three above honest.** A dialog that refused every
  // press would pass all of them, and a refusal is only correct if the
  // deliberate move to the other control still lands the work.
  await userEvent.click(confirmMerge());

  expect(sent.merged).toEqual([JOB.id]);
  await expect.element(page.getByRole("dialog")).not.toBeInTheDocument();
});

test("approve and request changes send on the press, with no dialog", async () => {
  const sent = gate();

  await userEvent.click(page.getByRole("button", { name: "Approve the work" }));
  expect(sent.approved).toEqual([JOB.id]);

  // Requesting changes is refused while the note is blank, which is what Fleet
  // would answer — so it is written before it is pressed.
  await userEvent.fill(
    page.getByRole("textbox", { name: "What should change" }),
    "The gate arm is missing from config's loader.",
  );
  await userEvent.click(page.getByRole("button", { name: "Request changes" }));
  expect(sent.changes).toEqual([JOB.id]);

  await expect.element(page.getByRole("dialog")).not.toBeInTheDocument();
});

test("reject still asks, and merging is not what it asks about", async () => {
  const sent = gate();

  await userEvent.click(page.getByRole("button", { name: "Reject the work" }));

  // One question at a time: the two confirmations share a state, and a shape
  // that let both stand would put two layers over one gate.
  await expect.element(page.getByRole("dialog")).toBeVisible();
  await expect.element(confirmMerge()).not.toBeInTheDocument();
  expect(sent.rejected, "the press rejected").toEqual([]);

  await userEvent.click(page.getByRole("dialog").getByRole("button", { name: "Reject the work" }));
  expect(sent.rejected).toEqual([JOB.id]);
});
