import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fireEvent, fn } from "storybook/test";
import { Sheet } from "./Sheet";

const meta: Meta<typeof Sheet> = {
  title: "Primitives/Sheet",
  component: Sheet,
};
export default meta;

type Story = StoryObj<typeof Sheet>;

/**
 * The contract names no Sheet state. It names one surface treatment and no
 * side, no width, no use. This story exists so the surface treatment can be
 * seen; what a sheet is for on Bridge is in the report as an open item — the
 * layout model says full-width routes with no inspector pane and no modal for
 * job detail, which rules out the two things a sheet usually does.
 */
export const Right: Story = {
  args: {
    open: true,
    side: "right",
    title: "Kit allowlist",
    children:
      "The command tripped the allowlist 5 times and was approved every time. Adding it here stops the prompt.",
    onClose: fn(),
  },
  /**
   * **Two exits and no third**, which is a rule about what does *not* happen
   * and so is the kind only a run can hold. A dialog's scrim usually closes
   * it; this one takes no press, because a 1676-entry read must not be lost to
   * a stray click beside the panel.
   *
   * The negative assertion is the valuable half. Adding an `onClick` to the
   * scrim is a one-line change that looks like a fix and would render
   * identically — nothing else here would notice.
   */
  play: async ({ args, canvas, userEvent }) => {
    const panel = canvas.getByRole("dialog", { name: "Kit allowlist" });

    // The ground behind, reached through the structure rather than by class
    // name: the scrim is what the panel renders inside, and it carries no role
    // on purpose because it is not a control.
    const ground = panel.parentElement;
    if (ground === null) throw new Error("the sheet renders inside its scrim");

    // Dispatched rather than clicked. A pointer press has to land somewhere,
    // and every point on the scrim is either the panel or bare ground whose
    // position depends on the window — this is the press the scrim would have
    // to handle, put straight on it.
    fireEvent.click(ground);
    await expect(args.onClose).not.toHaveBeenCalled();

    // The second exit. The first is the close control, which is a button and
    // needs no assertion to prove it is one.
    await userEvent.keyboard("{Escape}");
    await expect(args.onClose).toHaveBeenCalled();
  },
};

export const Left: Story = {
  args: {
    open: true,
    side: "left",
    title: "Kit allowlist",
    children:
      "The command tripped the allowlist 5 times and was approved every time. Adding it here stops the prompt.",
  },
};
