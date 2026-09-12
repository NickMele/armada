// Opening the run sheet from the panel, by pressing the control a person presses.
//
// **The defect this exists for.** `Run…` on the worktree row is wired
// straight to its handler; React calls a click handler with the click
// event. The panel passed `useRunSheet`'s `open` bare, so the event arrived
// where an entry id belongs and the next render asked it for `.indexOf` —
// `id.indexOf is not a function`, thrown inside `JobDetail`'s own render,
// the whole screen replaced by the boundary.
//
// It typechecked the whole way: `(entryId?: string) => void` is assignable
// to `() => void`, since a handler may ignore what it is passed. No type
// catches it, so a press has to.
//
// **Pressed rather than called.** Every existing test hands it `fn()`, a
// mock that swallows an argument it never reads — exactly why none saw this.
import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { JobDetail } from "./JobDetail";
import { running } from "./fixtures/build/running";
import { propsFor } from "./fixtures/props";
import { mount, unmount } from "./mounted";

afterEach(unmount);

test("pressing Run… opens the sheet instead of selecting the click event", async () => {
  mount(<JobDetail {...propsFor(running())} />);

  const run = page.getByRole("button", { name: "Run…" });
  await expect.element(run).toBeVisible();
  await userEvent.click(run);

  // The sheet is up, which is the whole of what the press is for.
  await expect.element(page.getByRole("dialog")).toBeVisible();
  // And the panel is still the panel: the crash replaced it with the boundary,
  // so the step's own heading going missing is the failure this catches.
  await expect.element(page.getByText("Where this step is")).toBeVisible();
});
