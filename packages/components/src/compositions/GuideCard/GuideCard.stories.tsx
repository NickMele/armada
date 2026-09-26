import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { GUIDE_ALWAYS_LOOKS, GUIDE_COMPLETION, GUIDE_MEMBER_LINK } from "../../guides";
import { GuideCard } from "./GuideCard";

const meta: Meta<typeof GuideCard> = {
  title: "Compositions/Guide card",
  component: GuideCard,
  args: { guide: GUIDE_COMPLETION, onClose: fn() },
};
export default meta;

type Story = StoryObj<typeof GuideCard>;

/** Pressed for. It scales in behind its scrim, the way summoned chrome may. */
export const Asked: Story = {};

/** Ten steps rather than eight. The list is the part that gives. */
export const Longer: Story = {
  args: { guide: GUIDE_ALWAYS_LOOKS },
};

/** A guide that carries a drawing, in the narrowest layer it is drawn in. */
export const WithAFigure: Story = {
  args: { guide: GUIDE_MEMBER_LINK },
};

/**
 * Opened by itself, on a piece nobody has met. **Nothing animates in**: no
 * person summoned this, so the contract's one licence for an entrance does not
 * reach it.
 */
export const Uninvited: Story = {
  args: { invited: false },
};

/**
 * **The first card anybody ever sees carries the switch.** It is what pays for
 * the card arriving uninvited at all, and it is the only card that draws one.
 */
export const TheFirstOneEver: Story = {
  args: { invited: false, off: false, onOff: fn(), onReadAll: fn() },
  play: async ({ args }) => {
    const card = within(document.body).getByRole("dialog", {
      name: `Guide ${GUIDE_COMPLETION.number}, ${GUIDE_COMPLETION.title}`,
    });
    const control = within(card).getByRole("switch", {
      name: /Open a guide the first time I meet a piece/,
    });
    await expect(control).toBeChecked();

    await userEvent.click(control);
    await expect(args.onOff).toHaveBeenCalledWith(true);
  },
};

/** The switch, once it has been turned off. The card says what still works. */
export const TurnedOff: Story = {
  args: { off: true, onOff: fn() },
};

/**
 * Every card after the first. **No switch**, because the offer was made once
 * and Settings is where it lives afterwards.
 */
export const EverySubsequentCard: Story = {
  args: { onReadAll: fn() },
  play: async () => {
    const card = within(document.body).getByRole("dialog", {
      name: `Guide ${GUIDE_COMPLETION.number}, ${GUIDE_COMPLETION.title}`,
    });
    await expect(within(card).queryByRole("switch")).toBeNull();
  },
};
