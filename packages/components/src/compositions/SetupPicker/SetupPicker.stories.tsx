import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { SetupPicker, type SetupPickerRow } from "./SetupPicker";

/**
 * Setup's picker over a storefront monorepo: two strong workspaces ticked, a thin one not,
 * and `lint` missing from the one sibling that does not declare it.
 */
const meta: Meta<typeof SetupPicker> = {
  title: "Compositions/Setup picker",
  component: SetupPicker,
};
export default meta;

type Story = StoryObj<typeof SetupPicker>;

const ROWS: SetupPickerRow[] = [
  { dir: ".", ticked: true, open: false, files: ["package.json", "pnpm-lock.yaml"], state: "ready to write" },
  { dir: "apps/web", ticked: true, open: true, files: ["apps/web/package.json"], state: "open, being edited" },
  { dir: "services/api", ticked: true, open: false, files: ["services/api/package.json"], state: "ready to write" },
  { dir: "docs", ticked: false, open: false, files: [], note: "No file here names anything runnable.", state: "no checks proposed" },
];

/** Ticked by evidence, one row open, and a gap marked narrowly. */
export const Picking: Story = {
  args: {
    rows: ROWS,
    onTick: fn(),
    onOpen: fn(),
    names: ["test", "lint", "typecheck"],
    grid: [
      { dir: ".", cells: ["declared", "absent", "absent"] },
      { dir: "apps/web", cells: ["declared", "declared", "declared"] },
      { dir: "services/api", cells: ["declared", "missing", "absent"] },
    ],
  },
  play: async ({ args, canvasElement, userEvent }) => {
    const canvas = within(canvasElement);
    const list = canvas.getByRole("region", { name: "Workspaces" });
    const docs = within(list).getByRole("listitem", { name: "docs" });
    await expect(within(docs).getByRole("checkbox")).not.toBeChecked();
    await expect(within(list).getByRole("listitem", { name: "apps/web" })).toHaveAttribute("aria-current", "true");

    // Ticking and opening are two acts: a tick opens nothing.
    await userEvent.click(within(docs).getByRole("checkbox"));
    await expect(args.onTick).toHaveBeenCalledWith("docs", true);
    await expect(args.onOpen).not.toHaveBeenCalled();
    await userEvent.click(within(docs).getByRole("button", { name: "Open docs" }));
    await expect(args.onOpen).toHaveBeenCalledWith("docs");

    const grid = canvas.getByRole("region", { name: "Check names across the batch" });
    await expect(within(grid).getAllByText("missing")).toHaveLength(1);
  },
};

/** Nothing ticked proposes a Check, and the grid says so rather than drawing an empty table. */
export const NoChecksInTheBatch: Story = {
  name: "No Checks in the batch",
  args: { rows: [ROWS[3]!], onTick: fn(), onOpen: fn(), names: [], grid: [] },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByText("No ticked workspace proposes a Check.")).toBeVisible();
  },
};
