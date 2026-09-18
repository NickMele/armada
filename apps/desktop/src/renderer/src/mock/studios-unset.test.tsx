// Studios on All repositories, through `App`, against what is set up and what is not: no
// repository with a Manifest, which is the dead end the surface used to draw as a picker with
// nothing pickable in it, and the mixture, where the picker holds both halves at once.
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
    .element(page.getByText("No repository is set up yet, so none of them keeps Studios."))
    .toBeVisible();
  await expect
    .element(page.getByText("A Studio is kept against a repository's Manifest. Set one up, and its Studios open here."))
    .toBeVisible();
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
  expect(page.getByText("No repository is set up yet, so none of them keeps Studios.").query()).toBeNull();
});

test("every-state serves one repository nobody set up, so the picker draws it greyed beside the pickable ones", async () => {
  // The state the greyed-out half was written for, and until `every-state` served an unset
  // repository nobody browsing the mock reached it: the two groups in one open picker.
  const scenario = scenarioNamed("every-state")!;
  mount({ ...scenario, state: { ...scenario.state, repository: null } });
  await openStudios();
  await expect.element(picker()).toBeVisible();

  // The greyed half, read by what its rows say. **Not by the Not set up group itself**: Chromium
  // gives a closed select's `optgroup` no accessible role, so `getByRole("group")` finds nothing
  // here — `packages/screens/src/AskRepository.test.tsx` is where the grouping is held. Every row
  // in it carries the same instruction and nothing else on the surface does, and an unset
  // repository's own label is its folder, which the scenario is free to change.
  const unset = picker()
    .getByRole("option")
    .elements()
    .filter((one) => (one.textContent ?? "").includes("Set it up first"));
  expect(unset.length).toBeGreaterThan(0);
  for (const option of unset) expect(option).toBeDisabled();

  // The pickable half, in the same picker — which is the whole point of the moment. Past the
  // placeholder, which is disabled too and is not a repository.
  const pickable = picker()
    .getByRole("option")
    .elements()
    .filter((one) => !unset.includes(one) && one.textContent?.trim() !== "Choose a repository");
  expect(pickable.length).toBeGreaterThan(0);
  for (const option of pickable) expect(option).toBeEnabled();
});
