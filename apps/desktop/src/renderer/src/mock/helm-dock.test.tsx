// What Helm's dock says while it is pointed at no repository, through `App`.
//
// `helm.state === "none"` is *Helm is pointed at nothing*, and `main/helm.ts`
// publishes it for several reasons — a rail sitting on a repository that is
// not set up is one, with others set up beside it. The dock drew one sentence
// for all of them, "No repository has a Manifest yet for Helm to answer
// about.", which on `every-state` told a person a false fact about their own
// machine on the one surface whose job is answering about it.
//
// Every mock scenario publishes `helm: { state: "none" }` — no main is behind
// this window to point it — so the dock's unpointed sentence is what these two
// moments draw without anything being pressed.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

/** The sentence that stood for every unpointed state, and was true in one of them. */
const ONCE_SAID = "No repository has a Manifest yet for Helm to answer about.";

/** The dock beside the content — an `aside` of its own, labelled Helm. */
const dock = () => page.getByRole("complementary", { name: "Helm" });

test("repositories set up and Helm pointed at none: the dock counts them, and the switch that points it is there", async () => {
  // Two of this scenario's repositories are set up, and nothing has pointed Helm at either.
  mount("every-state");
  await expect.element(dock()).toBeVisible();

  await expect
    .element(page.getByText("Helm is not pointed at a repository. 2 are set up, so pick one to ask about it."))
    .toBeVisible();
  expect(page.getByText(ONCE_SAID).query()).toBeNull();
  // The act the sentence names is the dock's own switch, under the thread it is written in.
  await expect.element(page.getByRole("combobox", { name: "Point Helm at a different repository" })).toBeVisible();
});

test("nothing set up: the dock says so in the words the rail and Setup use, and names the way out", async () => {
  // `nothing-set-up` is two repositories served and neither of them set up.
  mount("nothing-set-up");
  await expect.element(dock()).toBeVisible();

  await expect
    .element(
      page.getByText("Nothing is set up yet for Helm to answer about. Set up a repository, and Helm answers for it."),
    )
    .toBeVisible();
  // Neither the old sentence nor the pointed-at-none one, which would be false here.
  expect(page.getByText(ONCE_SAID).query()).toBeNull();
  expect(page.getByText(/Helm is not pointed at a repository/).query()).toBeNull();
});
