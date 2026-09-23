import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, waitFor, within } from "storybook/test";

import { GuidanceProvider } from "../../guidance";
import { GUIDE_GROUP_BOUNDARY, GUIDE_PROCESSES } from "../../guides";
import { GuideMark } from "./GuideMark";

/**
 * The `?` beside a piece. **Wrapped in a provider that remembers nothing**, so
 * a story never decides what the next window has already met.
 */
const meta: Meta<typeof GuideMark> = {
  title: "Compositions/Guide mark",
  component: GuideMark,
  decorators: [
    (Story) => (
      <GuidanceProvider remembered={false}>
        <div style={{ padding: "var(--space-6)", background: "var(--surface-canvas)" }}>
          <Story />
        </div>
      </GuidanceProvider>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof GuideMark>;

/**
 * Beside the label it explains. The heading is the fact; the mark is the way
 * to ask what it means.
 */
export const BesideALabel: Story = {
  args: { guide: GUIDE_GROUP_BOUNDARY, onScreen: false },
  render: (args) => (
    <p style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", margin: 0 }}>
      <span>7 checks ran at this boundary</span>
      <GuideMark {...args} />
    </p>
  ),
};

/**
 * **The mark opens its card.** The claim the whole system rests on: a press
 * puts that guide on screen, with its number and its title.
 */
export const OpensItsCard: Story = {
  args: { guide: GUIDE_GROUP_BOUNDARY, onScreen: false },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    const guide = args.guide;
    await userEvent.click(canvas.getByRole("button", { name: `Open guide ${guide.number}, ${guide.title}` }));

    // The layer renders under the provider rather than inside the mark, so it
    // is found on the body and not on the story's own element.
    const card = within(document.body).getByRole("dialog", {
      name: `Guide ${guide.number}, ${guide.title}`,
    });
    // A card somebody pressed for scales in from opacity 0, so it is in the
    // tree a frame before it can be seen. Waiting is the assertion: a card
    // that never reaches full opacity is one nobody can read.
    await waitFor(() => expect(card).toBeVisible());
    await expect(within(card).getByText(guide.body[0] as string)).toBeVisible();

    await userEvent.click(within(card).getByRole("button", { name: "Close" }));
    await expect(
      within(document.body).queryByRole("dialog", { name: `Guide ${guide.number}, ${guide.title}` }),
    ).toBeNull();
  },
};

/**
 * **One card by itself, whatever else is on the screen.** Two pieces nobody
 * has met arrive together; the first opens and the second waits for a later
 * visit. Four cards in a row is the noise the mark exists to replace.
 */
export const OnlyOneOpensItself: Story = {
  args: { guide: GUIDE_GROUP_BOUNDARY },
  render: (args) => (
    <p style={{ display: "flex", alignItems: "center", gap: "var(--space-2)", margin: 0 }}>
      <span>Two pieces, both new</span>
      <GuideMark {...args} />
      <GuideMark guide={GUIDE_PROCESSES} />
    </p>
  ),
  play: async () => {
    const body = within(document.body);
    await expect(body.getAllByRole("dialog")).toHaveLength(1);
    const first = body.getByRole("dialog", {
      name: `Guide ${GUIDE_GROUP_BOUNDARY.number}, ${GUIDE_GROUP_BOUNDARY.title}`,
    });
    await expect(first).toBeVisible();

    // Closed, and nothing takes its place: the second piece keeps its own
    // first contact rather than being spent in the same breath.
    await userEvent.click(within(first).getByRole("button", { name: "Close" }));
    await expect(body.queryAllByRole("dialog")).toHaveLength(0);
  },
};
