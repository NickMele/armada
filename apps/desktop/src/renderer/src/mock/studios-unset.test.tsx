// Studios on All repositories, through `App`, when no repository has a Manifest — the dead end the
// surface used to draw as a picker with nothing pickable in it.
//
// **Beside `studios.test.tsx` rather than in it**: that file is #1287's and #1341's definition of
// done, and this is one state neither of them names.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, unmountAfterEach } from "./testing";
import { scenarioNamed } from "./scenario";

unmountAfterEach();

/** Studios, from the rail, on a window that opens on Overview. */
async function openStudios(): Promise<void> {
  await page.getByRole("button", { name: "Studios", exact: true }).first().click();
}

/**
 * The ask's own select, which is the picker this state must not draw. **Named exactly**: Helm's
 * footer draws a second combobox, *Point Helm at a different repository*, which a loose name
 * matches too.
 */
const picker = () => page.getByRole("combobox", { name: "Repository", exact: true });

test("no repository set up: Studios says so and offers Setup, rather than a picker holding nothing", async () => {
  // `nothing-set-up` is the moment: repositories served, none of them with a Manifest, and the
  // rail on All repositories because nobody picked one.
  mount("nothing-set-up");
  await openStudios();

  await expect
    .element(page.getByText("No repository has a Manifest yet, so none of them keeps Studios."))
    .toBeVisible();
  await expect.element(page.getByText("Set one up, and this surface opens on its Studios.")).toBeVisible();
  // Not an empty question: no select at all, and no ask that would have held one.
  expect(picker().query()).toBeNull();
  expect(page.getByText("Pick a repository to open its Studios").query()).toBeNull();

  // The way forward is Bridge's existing one: the Manifest surface, whose own ask picks a
  // repository, and picking one with no Manifest opens Setup on it.
  await page.getByRole("button", { name: "Set up a repository" }).click();
  await expect.element(page.getByText("Pick a repository to open its Manifest")).toBeVisible();
});

test("every-state has repositories with Manifests, so Studios keeps its picker", async () => {
  // The other half of the rule: the ask stands wherever a repository could answer it.
  const scenario = scenarioNamed("every-state")!;
  mount({ ...scenario, state: { ...scenario.state, repository: null } });
  await openStudios();

  await expect.element(page.getByText("Pick a repository to open its Studios")).toBeVisible();
  await expect.element(picker()).toBeVisible();
  // The whole sentence: Helm's dock opens on one of its own that starts the same way.
  expect(page.getByText("No repository has a Manifest yet, so none of them keeps Studios.").query()).toBeNull();
});
