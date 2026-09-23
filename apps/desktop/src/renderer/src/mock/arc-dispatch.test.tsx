// The milestone's claim for the two moments before a Job exists: typing the
// prompt, and sketching beside it. `arc.test.tsx` is the rest of the arc, and
// this is a sibling of it because that file reached the length the gate
// refuses. The rule there holds here — a claim names what a person sees, never
// a component.

import { expect, test, describe } from "vitest";
import { page } from "vitest/browser";

import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

describe("dispatch", () => {
  test(
    "arc/dispatch-typing: the prompt a person is typing stands beside the two Jobs already " +
      "writing where this work would, each named with the path they share — and nothing is " +
      "greyed out, because an overlap is a fact and not a refusal",
    async () => {
      mount("arc/dispatch-typing");

      const field = page.getByRole("textbox", { name: "Request" });
      await expect.element(field).toHaveValue(expect.stringContaining("Drones 1 of 2"));

      const beside = page.getByText("What else is running").first();
      await expect.element(beside).toBeVisible();
      await expect
        .element(page.getByText("Fold the capacity read into one query"))
        .toBeVisible();
      // The chip's title is the whole path, which is what a person reads on
      // hover and what survives the directory's own clip.
      await expect.element(page.getByTitle("crates/api/src/")).toBeVisible();
      await expect.element(page.getByText("Give the rail its own scroll")).toBeVisible();
      await expect.element(page.getByTitle("packages/screens/src/overview.ts")).toBeVisible();

      // The claim the panel exists for: it says what is running and stops
      // nothing. A greyed Dispatch would make the reading a refusal.
      await expect
        .element(page.getByRole("button", { name: "Dispatch", disabled: true }))
        .not.toBeInTheDocument();
    },
  );

  test(
    "arc/dispatch-typing: the form says how many Drones this Job may run at once, and what " +
      "the machine allows across every Job, as two different numbers",
    async () => {
      mount("arc/dispatch-typing");
      // The block's own head, which says how many are set — the rail carries a
      // Settings of its own.
      await page.getByRole("button", { name: /Settings \d set/ }).click();

      await expect.element(page.getByRole("spinbutton", { name: "Drones at once" })).toHaveValue(2);
      await expect.element(page.getByText("This machine runs 4 at once")).toBeVisible();
    },
  );

  test(
    "arc/dispatch-typing: where the work starts and where it lands are two fields, and both " +
      "read main without either one being called the other's default",
    async () => {
      mount("arc/dispatch-typing");

      await expect.element(page.getByRole("combobox", { name: "From" })).toHaveValue("main");
      await expect.element(page.getByRole("combobox", { name: "Lands in" })).toHaveValue("main");
      // Neither field says what the other does. A parenthetical default is how
      // the pair collapses back into the one field it used to be.
      await expect.element(page.getByText("default", { exact: false })).not.toBeInTheDocument();
    },
  );

  test(
    "arc/dispatch-typing: both ref fields offer the repository's branches with the base " +
      "marked, and only where it lands offers to make one that is not there yet",
    async () => {
      mount("arc/dispatch-typing");

      const landsIn = page.getByRole("combobox", { name: "Lands in" });
      await expect.element(landsIn).toBeVisible();
      await landsIn.click();

      // The base leads and says it is the base; every other branch says which
      // Job is sitting on it.
      await expect.element(page.getByRole("option", { name: "main base" })).toBeVisible();
      await expect
        .element(
          page.getByRole("option", {
            name: "armada/18-fold-the-capacity-read Fold the capacity read into one query",
          }),
        )
        .toBeVisible();

      // Typing a name nothing has cut is the whole of creating a branch: one
      // row, and taking it is agreeing to it.
      await landsIn.fill("release/16");
      const make = page.getByRole("option", { name: "release/16 new branch" });
      await expect.element(make).toBeVisible();
      await make.click();
      await expect.element(landsIn).toHaveValue("release/16");

      // Where the work starts makes none — you cannot begin on a branch that
      // does not exist — and the typed name still stands, because the list is
      // what Armada has met rather than the repository's own.
      const from = page.getByRole("combobox", { name: "From" });
      await from.fill("release/17");
      await expect.element(page.getByText(/No branch Armada has met matches/)).toBeVisible();
      await expect
        .element(page.getByRole("option", { name: /new branch/ }))
        .not.toBeInTheDocument();
      await expect.element(from).toHaveValue("release/17");
    },
  );
  test(
    "arc/dispatch-sketch: the picture a person drew is on screen beside the prompt, with what " +
      "they said about it and the Studio node it was made from",
    async () => {
      mount("arc/dispatch-sketch");

      // Beside the prompt: the words are in the field and the picture is
      // attached to them, with where it was made read before its name.
      await expect
        .element(page.getByRole("textbox", { name: "Request" }))
        .toHaveValue(expect.stringContaining("Drones 1 of 2"));
      await expect.element(page.getByText("From a Studio")).toBeVisible();
      await expect.element(page.getByText("sketch 1")).toBeVisible();

      await page.getByRole("tab", { name: "Sketch" }).click();

      // The picture itself: three boxes and the two lines between them.
      await expect.element(page.getByRole("group", { name: "Box: Drones 1 of 2" })).toBeVisible();
      await expect
        .element(page.getByRole("group", { name: /^Box: a panel under it/ }))
        .toBeVisible();
      await expect
        .element(page.getByRole("group", { name: /^Box: the Job and the step/ }))
        .toBeVisible();
      expect(page.getByRole("group", { name: /^A line from / }).elements()).toHaveLength(2);

      // What they said about it, and the node it was made from.
      await expect
        .element(page.getByRole("textbox", { name: "About this sketch" }))
        .toHaveValue(expect.stringContaining("The panel opens under the stat"));
      await expect.element(page.getByText(/Made from\s+rail-stats\s+in a Studio/)).toBeVisible();
    },
  );

  test(
    "arc/dispatch-sketch: the words typed under Write are still there after a trip through " +
      "Sketch and back",
    async () => {
      mount("arc/dispatch-sketch");
      const field = page.getByRole("textbox", { name: "Request" });
      await expect.element(field).toBeVisible();
      const before = (field.element() as HTMLTextAreaElement).value;

      await page.getByRole("tab", { name: "Sketch" }).click();
      await expect.element(page.getByRole("textbox", { name: "About this sketch" })).toBeVisible();
      expect(page.getByRole("textbox", { name: "Request" }).query()).toBeNull();

      await page.getByRole("tab", { name: "Write" }).click();
      await expect.element(page.getByRole("textbox", { name: "Request" })).toHaveValue(before);
    },
  );
});
