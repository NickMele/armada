// What `helm-talking` exists to make reachable, through `App`.
//
// The dock's own controls had no scenario behind them: every moment published
// `helm: { state: "none" }`, so the thread, the composer in its pointed state
// and the record's split button could only be seen by a test building the
// state inline — which is how #1488 came to stand up a scenario, screenshot
// through it and delete it again. This asserts the three things the scenario
// was landed for, so a scenario quietly narrowed back to an empty thread fails
// here rather than at the next person's screenshot.
//
// `helm-dock.test.tsx` beside this owns the *unpointed* dock and *Start fresh*.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { entered, mount, unmountAfterEach } from "./testing";

unmountAfterEach();

const dock = () => page.getByRole("complementary", { name: "Helm" });

test("the dock opens on a conversation, both voices in it and the answer that never came", async () => {
  mount("helm-talking");
  await expect.element(dock()).toBeVisible();

  // The person's ask, and the reply rendered as the prose it is — each of
  // Helm's paragraphs its own block, not one run-on line.
  await expect.element(page.getByText("why did 77 stop")).toBeVisible();
  await expect
    .element(
      page.getByText("Job 77 stopped on step 4 of 6, Regression check. cargo_nextest failed on all three attempts."),
    )
    .toBeVisible();
  await expect.element(page.getByText("Produced — exit 101 — 3 of 2034 tests failed")).toBeVisible();
  // The reply's cost, which every reply carries.
  await expect.element(page.getByText("~$0.02 · 4 turns")).toBeVisible();

  // And the failure the record's controls exist for — a second ask with
  // nothing behind it. Without this the split button stands in no moment.
  // Fleet's own sentence, whole and unframed on both surfaces — the thread's
  // "No reply came. " and the record's "no reply came: " are gone. Read off
  // the row's whole text, because a substring match reads the same either way.
  const failed = page.getByText("Helm's door would not be configured: Permission denied (os error 13)");
  await expect.element(failed).toBeVisible();
  expect(failed.element().textContent).toBe("Helm's door would not be configured: Permission denied (os error 13)");

  // Bridge's own line, immediately above Fleet's, in the case Fleet's reads.
  // The two rows a thread writes about itself are the only place two producers
  // meet in one column, and the split the owner saw was here. Whole text
  // again, because a substring match reads the same in either case.
  const fresh = page.getByText("the stored session could not be resumed, so this reply starts a new one");
  await expect.element(fresh).toBeVisible();
  expect(fresh.element().textContent).toBe("the stored session could not be resumed, so this reply starts a new one");
});

test("pointed at a repository, the composer takes a message", async () => {
  mount("helm-talking");
  await expect.element(dock()).toBeVisible();

  const field = page.getByRole("textbox", { name: "Ask Helm" });
  await expect.element(field).toBeEnabled();
  // Send is refused on a blank field and offered on a typed one — the whole of
  // what the composer decides, and unreachable while Helm is pointed at nothing.
  const send = page.getByRole("button", { name: /^Send/ });
  await expect.element(send).toBeDisabled();
  await field.fill("has this test ever passed on main");
  await expect.element(send).toBeEnabled();

  // No switch: pointed at the one repository set up, there is nothing to switch to.
  expect(page.getByRole("combobox", { name: /^Point Helm at a/ }).query()).toBeNull();
});

test("the record's split button offers both acts, and Details opens this session and no other", async () => {
  mount("helm-talking");
  await expect.element(dock()).toBeVisible();

  // The face is the act the pair exists for, and the caret carries the reading.
  await expect.element(page.getByRole("button", { name: "Copy debug info" })).toBeEnabled();
  await page.getByRole("button", { name: "More ways to report this answer" }).click();
  await page.getByRole("menuitem", { name: "Details" }).click();

  const sheet = page.getByRole("dialog", { name: "Session record" });
  await entered(sheet);
  const shown = page.getByText(/armada helm session/);
  await expect.element(shown).toBeVisible();

  // The record is the conversation on screen, from Fleet's side — one session,
  // not a second fixture that happens to be drawable.
  const text = shown.element().textContent ?? "";
  expect(text).toContain("why did 77 stop");
  expect(text).toContain("called get_job");
  expect(text).toContain("$0.0214 · 4 turns");
  expect(text).toMatch(/fleet +Helm's door would not be configured: Permission denied \(os error 13\)/);
  expect(text).not.toContain("no reply came: ");
  // The record's own column has read lower case since it was written, which is
  // the case the thread beside it now reads too — one rule, two surfaces.
  expect(text).toMatch(/fleet +the stored session was gone, so this reply started a new one/);
});
