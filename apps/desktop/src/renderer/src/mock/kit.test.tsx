// The Kit surface, through `App` — the MCP servers a person has connected, and
// which of them a Drone dispatched against the picked repository is handed.
// #1275. A rail surface at `⌘8` since the owner reversed its first placement as
// a Manifest tab.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { DRIFT_GONE, GH_ISSUE_VIEW, KIT_SERVERS, manifesting } from "./manifest-fleet";
import type { Manifesting } from "./manifest-fleet";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

/** The Kit surface, by the rail. */
async function kit(options: Manifesting = {}): Promise<void> {
  mount(manifesting(options));
  await page.getByRole("button", { name: "Kit", exact: true }).click();
}

/**
 * #1275's own claim, end to end and without a terminal: a person adds an MCP
 * server, sees that it reaches no Drone, allows it for this repository, and the
 * row says a Drone dispatched here gets it.
 */
test("a server added in Bridge reaches no Drone until this repository allows it", async () => {
  await kit();
  await expect.element(page.getByText(/Nothing in your Kit yet/)).toBeVisible();

  await userEvent.fill(page.getByRole("textbox", { name: "Name" }), "nexus");
  await userEvent.fill(page.getByRole("textbox", { name: "Program" }), "npx -y @scope/server");
  await page.getByRole("button", { name: "Add" }).click();

  await expect.element(page.getByText("npx -y @scope/server")).toBeVisible();
  await expect.element(page.getByText("Does not")).toBeVisible();

  await userEvent.selectOptions(page.getByRole("combobox", { name: "Here: nexus" }), "extended");
  await expect.element(page.getByText("Gets it")).toBeVisible();

  // And taking the server out takes this repository's word with it.
  await page.getByRole("button", { name: "Remove nexus" }).click();
  await expect.element(page.getByText(/Nothing in your Kit yet/)).toBeVisible();
});

/**
 * **Both tiers on one screen**, which is why Kit is one surface rather than a
 * machine-wide list and a per-repository one: the column that says what a Drone
 * gets is only readable beside the two answers it is over.
 */
test("Kit's default and this repository's word are read side by side", async () => {
  await kit({ alwaysAllowed: [GH_ISSUE_VIEW], drift: DRIFT_GONE, kitServers: KIT_SERVERS });

  // Kit leaves `tracker` off and this repository allowed it; Kit turned
  // `nexus` on everywhere and this repository withheld it.
  await expect.element(page.getByRole("combobox", { name: "Here: tracker" })).toHaveValue("extended");
  await expect.element(page.getByRole("combobox", { name: "In Kit: nexus" })).toHaveValue("yes");
  await expect.element(page.getByRole("combobox", { name: "Here: nexus" })).toHaveValue("restricted");
  expect(page.getByText("Gets it").elements()).toHaveLength(1);
});

/**
 * Kit is machine-wide and its second tier is a repository's, so on All
 * repositories there is nothing to narrow. The surface asks for one rather than
 * drawing a control that answers for nobody — the Manifest surface's own
 * arrangement.
 */
test("All repositories asks for one rather than drawing a tier that answers for nobody", async () => {
  await kit({ kitServers: KIT_SERVERS, picked: false });
  await expect.element(page.getByText(/Pick a repository to see what its Drones are handed/)).toBeVisible();
});
