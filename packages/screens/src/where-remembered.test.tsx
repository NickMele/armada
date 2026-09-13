// Where things are' open choice, read back across what `OneJob` itself does
// not survive — a relaunch, simulated here as a fresh mount with nothing
// carried over in memory. `job-switch.test.tsx` is the sibling proof for a
// Job switch within one running Bridge.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";

import { JobDetail, WHERE_OPEN_KEY } from "./JobDetail";
import { running } from "./fixtures/build/running";
import { propsFor } from "./fixtures/props";
import { mount, unmount } from "./mounted";

afterEach(unmount);

const where = () => page.getByRole("button", { name: /Where things are/i });

test("Where things are stays open across a relaunch, once a person opens it", async () => {
  // A clean slate: another test's write must not decide this one's default.
  localStorage.removeItem(`armada.${WHERE_OPEN_KEY}`);

  mount(<JobDetail {...propsFor(running())} />);
  await expect.element(where()).toHaveAttribute("aria-expanded", "false");
  await userEvent.click(where());
  await expect.element(where()).toHaveAttribute("aria-expanded", "true");
  unmount();

  // Nothing survives in memory across this — the next mount reads whatever
  // a relaunch would.
  mount(<JobDetail {...propsFor(running())} />);
  await expect.element(where()).toHaveAttribute("aria-expanded", "true");

  localStorage.removeItem(`armada.${WHERE_OPEN_KEY}`);
});

test("stays closed for a person who has never opened it", async () => {
  localStorage.removeItem(`armada.${WHERE_OPEN_KEY}`);
  mount(<JobDetail {...propsFor(running())} />);
  await expect.element(where()).toHaveAttribute("aria-expanded", "false");
});
