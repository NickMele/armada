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
// this window to point it — so the dock's unpointed sentence is what these
// moments draw without anything being pressed.

import { expect, test } from "vitest";
import { page } from "vitest/browser";

import { MANIFEST_ID, repository, workflow } from "@armada/screens/src/fixtures/build/base";
import { connected } from "./moment";
import type { Scenario } from "./moment";
import { mount, openHelm, unmountAfterEach } from "./testing";

unmountAfterEach();

/**
 * Helm pointed at a repository, with a reply either out or finished — the one
 * fact *Start fresh* is refused by.
 */
function talking(replying: boolean): Scenario {
  return {
    name: `helm-${replying ? "replying" : "idle"}`,
    says: replying ? "Helm is writing a reply" : "Helm is pointed at a repository and idle",
    state: {
      ...connected([], [workflow()], [repository()]),
      helm: { state: "open", manifestId: MANIFEST_ID, replying, skipped: 0, missed: 0, items: [] },
    },
    reads: {},
    behaves: () => ({}),
  };
}

/** The sentence that stood for every unpointed state, and was true in one of them. */
const ONCE_SAID = "No repository has a Manifest yet for Helm to answer about.";

test("repositories set up and Helm pointed at none: the dock counts them, and the switch that points it is there", async () => {
  // Two of this scenario's repositories are set up, and nothing has pointed Helm at either.
  mount("every-state");
  await openHelm();

  await expect
    .element(page.getByText("Helm is not pointed at a repository. 2 are set up, so pick one to ask about it."))
    .toBeVisible();
  expect(page.getByText(ONCE_SAID).query()).toBeNull();
  // The act the sentence names is the dock's own switch, under the thread it is written in.
  const switcher = page.getByRole("combobox", { name: "Point Helm at a repository" });
  await expect.element(switcher).toBeVisible();
  // And the switch agrees with the sentence: it stands at an entry of its own,
  // not at whichever repository is listed first. It read *armada* here — a
  // `<select>` whose value matches no option displays the first one — so the
  // dock said Helm was pointed at nothing while the control said armada.
  await expect.element(switcher).toHaveDisplayValue("Choose a repository");
});

test("one repository set up and Helm pointed at none: the dock names it, and there is a switch to pick it with", async () => {
  // `empty-store` is one repository set up, no Job yet, and nothing pointing Helm at it —
  // the moment the sentence named an act with no control under it, because the composer
  // counted the repositories and one is nothing to switch between.
  mount("empty-store");
  await openHelm();

  await expect
    .element(page.getByText("Helm is not pointed at a repository. Pick armada to ask about it."))
    .toBeVisible();
  const switcher = page.getByRole("combobox", { name: "Point Helm at a repository" });
  await expect.element(switcher).toBeVisible();
  // Standing at its own entry, with the repository the sentence names under it to pick.
  await expect.element(switcher).toHaveDisplayValue("Choose a repository");
  await expect.element(page.getByRole("option", { name: "armada" })).toBeInTheDocument();
});

test("nothing set up: the dock says so in the words the rail and Setup use, and names the way out", async () => {
  // `nothing-set-up` is two repositories served and neither of them set up.
  mount("nothing-set-up");
  await openHelm();

  await expect
    .element(
      page.getByText("Nothing is set up yet for Helm to answer about. Set up a repository, and Helm answers for it."),
    )
    .toBeVisible();
  // Neither the old sentence nor the pointed-at-none one, which would be false here.
  expect(page.getByText(ONCE_SAID).query()).toBeNull();
  expect(page.getByText(/Helm is not pointed at a repository/).query()).toBeNull();
  // And no switch, though the dock hands the composer its handler in every state: pointing
  // Helm at nothing is a reason to offer a repository, not a reason to offer none. The
  // sentence above is the whole of what this moment has to say.
  expect(page.getByRole("combobox", { name: /^Point Helm at a/ }).query()).toBeNull();
});

// Where Start fresh is drawn, and when Fleet refuses it — the owner's note of
// 18 Sep 2026. It ends the conversation the whole dock is showing, so it sits
// in the dock's own head beside Close rather than in the row above the message
// box, where it was one of three controls that wrapped onto a second line.

test("Start fresh sits in the dock's head, beside Close and above the conversation", async () => {
  mount(talking(false));
  await openHelm();

  const fresh = page.getByRole("button", { name: "Start fresh" });
  await expect.element(fresh).toBeVisible();
  await expect.element(fresh).toBeEnabled();

  // Beside Close, on the head's one line and ahead of it: the exit stays at
  // the trailing edge. No role carries where a control was drawn, and where
  // this one is drawn is the whole of the note.
  const act = fresh.element().getBoundingClientRect();
  const close = page.getByRole("button", { name: /^Close/ }).element().getBoundingClientRect();
  expect(Math.abs(act.top - close.top)).toBeLessThan(2);
  expect(act.right).toBeLessThanOrEqual(close.left);

  // And above the composer it left — the head of the dock, not the head of the
  // composer, which is the length of the conversation further down.
  const field = page.getByRole("textbox", { name: "Ask Helm" }).element().getBoundingClientRect();
  expect(act.bottom).toBeLessThan(field.top);
});

test("a reply is being written: Start fresh is refused, in the head where it now lives", async () => {
  mount(talking(true));
  await openHelm();

  const fresh = page.getByRole("button", { name: "Start fresh" });
  await expect.element(fresh).toBeDisabled();
  // Still the head's control while refused: a disabled act does not move.
  const act = fresh.element().getBoundingClientRect();
  const close = page.getByRole("button", { name: /^Close/ }).element().getBoundingClientRect();
  expect(Math.abs(act.top - close.top)).toBeLessThan(2);
});
