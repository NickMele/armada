import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { StudioName } from "./StudioName";

const meta: Meta<typeof StudioName> = {
  title: "Compositions/Studio name",
  component: StudioName,
  args: {
    name: "Stale counts",
    untitled: "Untitled Studio",
    editable: true,
    onRename: fn(),
  },
};
export default meta;

type Story = StoryObj<typeof StudioName>;

/** The heading over an open Studio. The heading opens nothing, so Rename is beside it. */
export const Heading: Story = {
  args: { heading: true },
};

/** Nobody has named it. Helm names an untitled Studio unasked, and until it does this is the word. */
export const Untitled: Story = {
  args: { heading: true, name: null },
};

/** A list row: the name is the way into the Studio, and the rename sits beside it. */
export const OnARow: Story = {
  args: { onOpen: fn() },
};

/** Reopened read-only. The name is drawn and there is no way to change it. */
export const ReadOnly: Story = {
  args: { heading: true, editable: false },
};

/** Out to Fleet: the field waits rather than taking a second press. */
export const Saving: Story = {
  args: { heading: true, saving: true },
  play: async ({ canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Rename Stale counts" }));
  },
};

/**
 * **The field opens on the name it is replacing, `Enter` sends it and `Esc`
 * abandons it** — three things a rendering cannot show. What `Esc` must not do
 * is send: a rename a person backed out of leaving the Studio renamed is the
 * one failure this states.
 */
export const Renaming: Story = {
  args: { heading: true },
  play: async ({ canvasElement, args }) => {
    const board = within(canvasElement);
    await userEvent.click(board.getByRole("button", { name: "Rename Stale counts" }));
    await expect(board.getByLabelText("Studio name")).toHaveValue("Stale counts");

    await userEvent.keyboard("{Escape}");
    await expect(args.onRename).not.toHaveBeenCalled();
    await expect(board.getByRole("heading", { name: "Stale counts" })).toBeVisible();

    // Typed without clicking in first: the field opens focused with the old
    // name selected, so the first keystroke replaces it.
    await userEvent.click(board.getByRole("button", { name: "Rename Stale counts" }));
    await userEvent.keyboard("The Board's legend{Enter}");
    await expect(args.onRename).toHaveBeenCalledWith("The Board's legend");
  },
};

/** A blank name is no name, and Fleet refuses one, so Save is off until there is one. */
export const NothingTyped: Story = {
  args: { heading: true, name: null },
  play: async ({ canvasElement, args }) => {
    const board = within(canvasElement);
    await userEvent.click(board.getByRole("button", { name: "Rename Untitled Studio" }));
    await userEvent.clear(board.getByLabelText("Studio name"));
    await expect(board.getByRole("button", { name: "Save" })).toBeDisabled();
    await userEvent.keyboard("   {Enter}");
    await expect(args.onRename).not.toHaveBeenCalled();
  },
};
