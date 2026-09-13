import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { OneFlag, OrderedPicks, ValuePopover } from "./ValuePopover";

/**
 * The one popover the proposal sheet opens from a value: a Check's prerequisites, picked and
 * ordered from the Commands the file declares, and a Command's destructive flag.
 */
const meta: Meta<typeof ValuePopover> = {
  title: "Compositions/Value popover",
  component: ValuePopover,
};
export default meta;

type Story = StoryObj<typeof ValuePopover>;

const picked = fn();

/** `e2e` runs `migrate` first. `reset` is destructive, so no Check may run it first. */
export const Prerequisites: Story = {
  args: {
    label: "e2e runs first",
    value: "migrate →",
    children: (
      <OrderedPicks
        label="Commands e2e runs first"
        says="Ticked Commands run before the Check, in the order ticked."
        options={[
          { name: "migrate" },
          { name: "seed" },
          { name: "reset", unavailable: "Destructive, so no Check may run it first." },
        ]}
        picked={["migrate"]}
        onPicked={picked}
      />
    ),
  },
  play: async ({ canvasElement, userEvent }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: "Edit e2e runs first" }));
    const layer = canvas.getByRole("dialog", { name: "e2e runs first" });
    await expect(within(layer).getByText("1")).toBeVisible();
    await expect(within(layer).getByRole("checkbox", { name: "reset" })).toBeDisabled();
    await userEvent.click(within(layer).getByRole("checkbox", { name: "seed" }));
    await expect(picked).toHaveBeenCalledWith(["migrate", "seed"]);
    // Never left up.
    await userEvent.keyboard("{Escape}");
    await expect(canvas.queryByRole("dialog")).toBeNull();
  },
};

const flagged = fn();

/** The set of one: a switch, what it changes, and whose call it is. */
export const Destructive: Story = {
  args: {
    label: "reset destructive",
    offer: "Mark destructive",
    offerName: "Mark destructive: reset",
    children: (
      <OneFlag
        checked={false}
        description="A Drone asks you before it runs this."
        judgement="Nothing Scan reads can tell what a script destroys, so this is your judgement."
        onChange={flagged}
      >
        Destructive
      </OneFlag>
    ),
  },
  play: async ({ canvasElement, userEvent }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: "Mark destructive: reset" }));
    const layer = canvas.getByRole("dialog", { name: "reset destructive" });
    await expect(within(layer).getByText(/this is your judgement/)).toBeVisible();
    await userEvent.click(within(layer).getByRole("switch", { name: /Destructive/ }));
    await expect(flagged).toHaveBeenCalledWith(true);
    await userEvent.keyboard("{Escape}");
    await expect(canvas.queryByRole("dialog")).toBeNull();
  },
};
