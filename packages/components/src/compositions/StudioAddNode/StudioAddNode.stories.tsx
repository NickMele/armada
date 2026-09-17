import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";

import { StudioAddNode } from "./StudioAddNode";

const meta: Meta<typeof StudioAddNode> = {
  title: "Compositions/Studio add node",
  component: StudioAddNode,
  args: {
    adding: null,
    onAdding: fn(),
    onAdd: fn(),
  },
};
export default meta;

type Story = StoryObj<typeof StudioAddNode>;

/** The control on an open Studio, closed. */
export const Control: Story = {};

/**
 * The three kinds a person adds by hand, each with the binding the registry
 * gives it. Nothing else is offered: every other kind is made by the act that
 * earns it, and Fleet refuses it from Bridge by name.
 */
export const Menu: Story = {
  play: async ({ canvasElement, args }) => {
    const board = within(canvasElement);
    await userEvent.click(board.getByRole("button", { name: /Node/ }));
    const items = board.getAllByRole("menuitem");
    await expect(items.map((item) => item.textContent)).toEqual(["NoteN", "LinkV", "SketchS"]);
    await userEvent.click(board.getByRole("menuitem", { name: /Note/ }));
    await expect(args.onAdding).toHaveBeenCalledWith("note");
  },
};

/** A Note typed here, which is fixed the moment it is made, as a captured one is. */
export const Note: Story = {
  args: { adding: "note" },
};

/** A Link pasted. One line, mono, because an address is machine-derived. */
export const Link: Story = {
  args: { adding: "link" },
};

/** A Sketch placed — structured content and never pixels, which the field says. */
export const Sketch: Story = {
  args: { adding: "sketch" },
};

/** Out to Fleet. The field waits rather than taking a second press. */
export const Adding: Story = {
  args: { adding: "note", saving: true },
};

/** Fleet did not take it, and the field says so without losing what was typed. */
export const Refused: Story = {
  args: { adding: "link", refused: "Fleet is not running, so nothing was added." },
};

/** A Studio reopened read-only. The control is there and does not open. */
export const ReadOnly: Story = {
  args: { disabled: true },
};

/**
 * **A line sends on `Enter` and prose does not** — a Note takes ⌘Enter, so a
 * paragraph break in one does not send it half-written. `Esc` abandons the
 * field rather than reaching the surface behind it, and a blank field sends
 * nothing at all.
 */
export const Writing: Story = {
  args: { adding: "note" },
  play: async ({ canvasElement, args }) => {
    const board = within(canvasElement);
    const field = board.getByLabelText("Note");

    await userEvent.type(field, "The legend is unreadable{Enter}");
    await expect(args.onAdd).not.toHaveBeenCalled();

    await userEvent.type(field, "{Meta>}{Enter}{/Meta}");
    // The newline `Enter` left is trimmed off: what is sent is what was written.
    await expect(args.onAdd).toHaveBeenCalledWith({ kind: "note", said: "The legend is unreadable" });

    await userEvent.clear(field);
    await expect(board.getByRole("button", { name: "Add note" })).toBeDisabled();
    await userEvent.keyboard("{Escape}");
    await expect(args.onAdding).toHaveBeenCalledWith(null);
  },
};
