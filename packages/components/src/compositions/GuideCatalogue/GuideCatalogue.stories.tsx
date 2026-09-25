import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, within } from "storybook/test";

import { GUIDES, GUIDE_GROUPS } from "../../guides";
import { GuideCatalogue } from "./GuideCatalogue";

/** The surface is bounded and each column scrolls itself, so the frame has a height. */
const meta: Meta<typeof GuideCatalogue> = {
  title: "Compositions/Guide catalogue",
  component: GuideCatalogue,
  decorators: [
    (Story) => (
      <div
        style={{
          display: "flex",
          height: "560px",
          padding: "var(--space-6)",
          background: "var(--surface-canvas)",
        }}
      >
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof GuideCatalogue>;

/** The row for a guide, named the way the card and the mark name it. */
const rowFor = (guide: { number: number; title: string }) => ({
  name: `Guide ${guide.number}, ${guide.title}`,
});

/**
 * **Every guide is reachable, and one is open on arrival.** The two claims the
 * surface exists to make: a person who met a word once finds it again, and the
 * panel is never the empty half of a two-column screen.
 */
export const EveryGuide: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    for (const guide of GUIDES) {
      await expect(canvas.getByRole("button", rowFor(guide))).toBeVisible();
    }
    const first = GUIDES[0];
    if (first === undefined) throw new Error("no guides");
    await expect(canvas.getByRole("heading", { name: first.title })).toBeVisible();
    await expect(canvas.getByRole("button", rowFor(first))).toHaveAttribute("aria-current", "true");
  },
};

/**
 * **A row's press opens that guide beside the list.** Asserted on the guide's
 * own body text rather than on its title, which the row also carries: a title
 * drawn twice would let a panel that never changed pass this.
 */
export const OpensWhatWasPressed: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const guide = GUIDES[7];
    if (guide === undefined) throw new Error("no eighth guide");
    const paragraph = guide.body[0];
    if (paragraph === undefined) throw new Error("no body");

    await expect(canvas.queryByText(paragraph)).toBeNull();
    await userEvent.click(canvas.getByRole("button", rowFor(guide)));
    await expect(canvas.getByText(paragraph)).toBeVisible();
    await expect(canvas.getByRole("button", rowFor(guide))).toHaveAttribute("aria-current", "true");

    // One at a time: the guide that was open on arrival is no longer marked.
    const first = GUIDES[0];
    if (first === undefined) throw new Error("no guides");
    await expect(canvas.getByRole("button", rowFor(first))).not.toHaveAttribute("aria-current");
  },
};

/**
 * **A deep link arrives on the guide it names.** What lets a `?` open the
 * catalogue at an entry rather than a card — and what `Read all guides` on a
 * card already does.
 */
export const ArrivesOnOne: Story = {
  args: { arriveAt: 11 },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const guide = GUIDES.find((one) => one.number === 11);
    if (guide === undefined) throw new Error("no guide 11");
    await expect(canvas.getByRole("heading", { name: guide.title })).toBeVisible();
    await expect(canvas.getByRole("button", rowFor(guide))).toHaveAttribute("aria-current", "true");
  },
};

/**
 * **Under `--layout-breakpoint` the list reaches the guide through a sheet.**
 * The list is what the surface arrives on, a press opens the guide over it,
 * and Close gives the list back — both halves reachable at a width that cannot
 * hold two columns.
 */
export const Narrow: Story = {
  args: { narrow: true },
  decorators: [
    (Story) => (
      <div
        style={{
          display: "flex",
          width: "var(--layout-breakpoint-narrow)",
          height: "560px",
          padding: "var(--space-6)",
          background: "var(--surface-canvas)",
        }}
      >
        <Story />
      </div>
    ),
  ],
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const guide = GUIDES[3];
    if (guide === undefined) throw new Error("no fourth guide");
    const paragraph = guide.body[0];
    if (paragraph === undefined) throw new Error("no body");

    // Arrives on the list: nothing is over it, and every row is pressable.
    await expect(canvas.queryByRole("dialog")).toBeNull();
    await userEvent.click(canvas.getByRole("button", rowFor(guide)));

    const sheet = canvas.getByRole("dialog", { name: guide.title });
    await expect(within(sheet).getByText(paragraph)).toBeVisible();

    await userEvent.click(within(sheet).getByRole("button", { name: /Close/ }));
    await expect(canvas.queryByRole("dialog")).toBeNull();
    // The choice outlives the sheet, so widening the window draws this guide.
    await expect(canvas.getByRole("button", rowFor(guide))).toHaveAttribute("aria-current", "true");
  },
};

/** One group, to see a section on its own. The list is the same shape at any length. */
export const OneGroup: Story = {
  args: { guides: GUIDES.filter((guide) => guide.group === "machine") },
};

/** Every group draws its own count, and the counts are the guides filed under it. */
export const GroupCounts: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    for (const group of GUIDE_GROUPS) {
      const section = canvas.getByRole("region", { name: group.title });
      const filed = GUIDES.filter((guide) => guide.group === group.id).length;
      // The count and a guide's own number are both bare digits, so the query
      // names the count itself rather than the first digit in the section.
      await expect(
        within(section).getByText(String(filed), { selector: ".armada-guides__count" }),
      ).toBeVisible();
    }
  },
};
