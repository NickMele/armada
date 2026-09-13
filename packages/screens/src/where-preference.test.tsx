// Where things are' open state, as Fleet's own preference — `#927`. This
// package stays free of Electron, so the region only ever draws what it is
// handed and only ever asks the caller to save a change; nothing here reads
// or writes storage of its own.

import { afterEach, expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";

import { JobDetail } from "./JobDetail";
import { running } from "./fixtures/build/running";
import { propsFor } from "./fixtures/props";
import { mount, unmount } from "./mounted";

afterEach(unmount);

const where = () => page.getByRole("button", { name: /Where things are/i });

test("opens when the preference is true", async () => {
  mount(<JobDetail {...propsFor(running())} whereOpen={true} />);
  await expect.element(where()).toHaveAttribute("aria-expanded", "true");
});

test("closed for a person who has never chosen", async () => {
  mount(<JobDetail {...propsFor(running())} whereOpen={false} />);
  await expect.element(where()).toHaveAttribute("aria-expanded", "false");
});

test("a press asks the caller to save the opposite of what is drawn", async () => {
  const onOpenWhere = vi.fn();
  mount(<JobDetail {...propsFor(running())} whereOpen={false} onOpenWhere={onOpenWhere} />);
  await userEvent.click(where());
  expect(onOpenWhere).toHaveBeenCalledTimes(1);
  expect(onOpenWhere).toHaveBeenCalledWith(true);
});
