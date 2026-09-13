import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, within } from "storybook/test";

import { SetupFrom } from "./Setup";

/**
 * Setup on the Manifest surface — Journey 3 — over a storefront nobody set up: the picker,
 * and each workspace's proposal opening over it and closing back to it, in any order.
 */
const meta = {
  title: "Screens/Setup",
  component: SetupFrom,
  parameters: { layout: "fullscreen" },
} satisfies Meta<typeof SetupFrom>;

export default meta;
type Story = StoryObj<typeof meta>;

/** Every workspace Scan found, ticked by evidence, with the gap the batch shows. */
export const ThePicker: Story = {
  name: "The picker",
  args: {},
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const list = await canvas.findByRole("region", { name: "Workspaces" });
    for (const dir of [".", "apps/web", "services/api", "docs"]) {
      await expect(within(list).getByRole("listitem", { name: dir })).toBeVisible();
    }
    await expect(within(within(list).getByRole("listitem", { name: "docs" })).getByRole("checkbox")).not.toBeChecked();
    await expect(within(within(list).getByRole("listitem", { name: "docs" })).getByText("no checks proposed")).toBeVisible();
    // This repository's own root already has a Manifest, so it is not offered as writable.
    await expect(within(within(list).getByRole("listitem", { name: "." })).getByText("already set up")).toBeVisible();
    // `lint` is declared by `apps/web` and not by `services/api`, its one ticked sibling.
    const grid = canvas.getByRole("region", { name: "Check names across the batch" });
    await expect(within(grid).getAllByText("missing")).toHaveLength(1);
  },
};

/**
 * A proposal opened, a line read for where it came from, a guess corrected, and the sheet
 * closed back to the picker, which then opens another. Not a wizard.
 */
export const CorrectAndClose: Story = {
  name: "Correct a line and close",
  args: {},
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const list = await canvas.findByRole("region", { name: "Workspaces" });
    await userEvent.click(within(list).getByRole("button", { name: "Open apps/web" }));
    const sheet = await canvas.findByRole("dialog", { name: "Proposal for apps/web" });
    const checks = within(sheet).getByRole("list", { name: "Checks" });
    const lint = within(checks).getByRole("listitem", { name: "lint" });
    await expect(within(lint).getByText("apps/web/package.json scripts.lint")).toBeVisible();
    await expect(within(lint).getByText("convention")).toBeVisible();

    await userEvent.click(within(lint).getByRole("button", { name: "Edit lint command" }));
    const field = within(lint).getByLabelText("lint command");
    await userEvent.clear(field);
    await userEvent.type(field, "pnpm eslint . --max-warnings 0{Enter}");
    await expect(await within(lint).findByText("edited during setup")).toBeVisible();
    await expect(within(lint).queryByText("convention")).toBeNull();

    // A guess moved in one press lands in the other registry.
    const dev = within(within(sheet).getByRole("list", { name: "Commands" })).getByRole("listitem", { name: "dev" });
    await userEvent.click(within(dev).getByRole("button", { name: "Move to Checks" }));
    await expect(await within(within(sheet).getByRole("list", { name: "Checks" })).findByRole("listitem", { name: "dev" })).toBeVisible();

    await userEvent.click(within(sheet).getByRole("button", { name: "Back to the workspaces" }));
    await expect(canvas.queryByRole("dialog")).toBeNull();
    await expect(within(within(list).getByRole("listitem", { name: "apps/web" })).getByText("ready to write")).toBeVisible();
    await userEvent.click(within(list).getByRole("button", { name: "Open services/api" }));
    const api = await canvas.findByRole("dialog", { name: "Proposal for services/api" });
    await expect(within(within(api).getByRole("list", { name: "Ports" })).getByText("compose.yaml services.db.ports")).toBeVisible();
  },
};

/** Write, and the sheet that wrote the file says what landed and which file Verify runs. */
export const Write: Story = {
  args: { write: "took" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const list = await canvas.findByRole("region", { name: "Workspaces" });
    await userEvent.click(within(list).getByRole("button", { name: "Open apps/web" }));
    const sheet = await canvas.findByRole("dialog", { name: "Proposal for apps/web" });
    await userEvent.click(within(sheet).getByRole("button", { name: "Write apps/web/armada.yml" }));
    await expect(await within(sheet).findByText(/^Wrote apps\/web\/armada.yml at /)).toBeVisible();
    await expect(within(sheet).getByText(/This file is not that one/)).toBeVisible();
    await expect(within(sheet).queryByRole("button", { name: /^Edit / })).toBeNull();
  },
};

/**
 * A root with an `armada.yml` already: its sheet offers no Write, and sends a person to the
 * Edit tab, which edits that file.
 */
export const AlreadySetUp: Story = {
  name: "A workspace already set up",
  args: {},
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const list = await canvas.findByRole("region", { name: "Workspaces" });
    await userEvent.click(within(list).getByRole("button", { name: "Open ." }));
    const sheet = await canvas.findByRole("dialog", { name: "Proposal for ." });
    await expect(within(sheet).getByText("armada.yml is already here")).toBeVisible();
    await expect(within(sheet).queryByRole("button", { name: "Write armada.yml" })).toBeNull();
    await userEvent.click(within(sheet).getByRole("button", { name: "Open the Edit tab" }));
    await expect(await canvas.findByRole("tab", { name: "Edit", selected: true })).toBeVisible();
  },
};

/** A root nobody set up lands on Verify after Write — the case a repository added by folder meets. */
export const WriteTheRoot: Story = {
  name: "Write the root, then Verify",
  args: { write: "took", rootSetUp: false, sheet: { state: "read", sheet: { setup: [], checks: [], commands: [] } } },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const list = await canvas.findByRole("region", { name: "Workspaces" });
    await userEvent.click(within(list).getByRole("button", { name: "Open ." }));
    const sheet = await canvas.findByRole("dialog", { name: "Proposal for ." });
    await userEvent.click(within(sheet).getByRole("button", { name: "Write armada.yml" }));
    const verify = await within(sheet).findByRole("region", { name: "Verify" });
    await expect(within(verify).getByRole("button", { name: "Verify" })).toBeEnabled();
  },
};

/** Write refused: the fault is under the row it names, and nothing was written. */
export const WriteRefused: Story = {
  name: "Write refused",
  args: { write: "refused" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(within(await canvas.findByRole("region", { name: "Workspaces" })).getByRole("button", { name: "Open apps/web" }));
    const sheet = await canvas.findByRole("dialog", { name: "Proposal for apps/web" });
    await userEvent.click(within(sheet).getByRole("button", { name: "Write apps/web/armada.yml" }));
    const lint = within(within(sheet).getByRole("list", { name: "Checks" })).getByRole("listitem", { name: "lint" });
    await expect(await within(lint).findByText("is empty.")).toBeVisible();
    await expect(within(sheet).getByText("Not written")).toBeVisible();
  },
};

/** A file was already at the path: nothing written over it, and the picker says so. */
export const AlreadyThere: Story = {
  name: "A file already there",
  args: { write: "appeared" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const list = await canvas.findByRole("region", { name: "Workspaces" });
    await userEvent.click(within(list).getByRole("button", { name: "Open apps/web" }));
    const sheet = await canvas.findByRole("dialog", { name: "Proposal for apps/web" });
    await userEvent.click(within(sheet).getByRole("button", { name: "Write apps/web/armada.yml" }));
    await expect(await within(sheet).findByText("Nothing was written over it. It reads:")).toBeVisible();
    await expect(within(sheet).queryByRole("button", { name: /^Write / })).toBeNull();
    await userEvent.click(within(sheet).getByRole("button", { name: "Back to the workspaces" }));
    await expect(within(within(list).getByRole("listitem", { name: "apps/web" })).getByText("already set up")).toBeVisible();
  },
};
