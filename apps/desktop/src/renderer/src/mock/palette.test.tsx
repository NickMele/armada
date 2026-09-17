// The command palette, through `App`: what it leaves out with no Job focused,
// and what it still draws dimmed. `docs/contracts/design-system.md`, Command
// palette.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import { running } from "@armada/screens/src/fixtures/build/index";
import { boardJobs, boardWorkflows } from "@armada/screens/src/fixtures/build/board";

import { onBoard, onJob } from "./scenario";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

/** ⌘K, and the palette's list once it is up. */
async function palette() {
  await userEvent.keyboard("{Meta>}k{/Meta}");
  const list = page.getByRole("dialog", { name: "Command palette" });
  await expect.element(list).toBeVisible();
  return list;
}

test("with no Job focused, an act on one Job is left out, and a row not built yet still draws", async () => {
  mount(onBoard(boardJobs(), { workflows: boardWorkflows() }));
  await expect.element(page.getByRole("heading", { name: "Running" }).first()).toBeVisible();
  const list = await palette();
  await expect.element(list.getByRole("option", { name: /^Pilot/ })).toBeInTheDocument();
  for (const verb of ["Open", "Review", "Attest", "Redirect", "Kill"]) {
    expect(list.getByRole("option", { name: new RegExp(`^${verb}\\b`) }).query()).toBeNull();
  }
  expect(list.getByText("no job focused", { exact: true }).query()).toBeNull();
});

test("with a Job open, its acts are back", async () => {
  const fixture = running();
  mount(onJob(fixture));
  await expect.element(page.getByText(fixture.job.handle, { exact: true }).first()).toBeVisible();
  const list = await palette();
  await expect.element(list.getByRole("option", { name: /^Kill/ }).first()).toBeInTheDocument();
});
