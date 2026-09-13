import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, within } from "storybook/test";

import { sheet } from "../Manifest/Manifest";
import { ManifestFormsFrom } from "./ManifestForms";

/**
 * The Manifest surface's **Edit** view — Journey 9, *Editing* — drawn from this
 * repository's own Checks and Commands, with Fleet faked just far enough to
 * write an edit, refuse one, and say what past Jobs cost.
 */
const meta = {
  title: "Screens/Manifest forms",
  component: ManifestFormsFrom,
  parameters: { layout: "fullscreen" },
} satisfies Meta<typeof ManifestFormsFrom>;

export default meta;
type Story = StoryObj<typeof meta>;

/**
 * **Edit** — the forms, the default editing view, with the file behind the tab
 * named by its path. Save writes the file and stops.
 */
export const TheForms: Story = {
  name: "The forms",
  args: { sheet: { state: "read", sheet: sheet() } },
};

/** A Check declared through the form and saved: the form redraws from what Fleet wrote. */
export const AddACheckAndSave: Story = {
  name: "Add a Check and save",
  args: { ...TheForms.args },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const checks = await canvas.findByRole("region", { name: "Checks" });
    await userEvent.type(within(checks).getByLabelText("New Check name"), "clippy");
    await userEvent.click(within(checks).getByRole("button", { name: "Add a Check" }));
    const clippy = within(checks).getByRole("group", { name: "clippy" });
    await userEvent.type(within(clippy).getByLabelText("Command"), "cargo clippy --workspace");
    await expect(canvas.getByText("Not saved.")).toBeVisible();

    await userEvent.click(canvas.getByRole("button", { name: "Save" }));
    await expect(await canvas.findByText(/^Saved /)).toBeVisible();
    await expect(within(checks).getByRole("group", { name: "clippy" })).toBeVisible();
    // Nothing left to save: the draft is the file as written.
    await expect(canvas.getByRole("button", { name: "Save" })).toBeDisabled();
  },
};

/** A Command removed. The writer takes its lines and the comment directly above them. */
export const RemoveACommand: Story = {
  name: "Remove a Command",
  args: { ...TheForms.args },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const commands = await canvas.findByRole("region", { name: "Commands" });
    const fmt = within(commands).getByRole("group", { name: "fmt" });
    await userEvent.click(within(fmt).getByRole("button", { name: "Remove" }));
    await expect(within(commands).queryByRole("group", { name: "fmt" })).toBeNull();

    await userEvent.click(canvas.getByRole("button", { name: "Save" }));
    await expect(await canvas.findByText(/^Saved /)).toBeVisible();
    await expect(within(commands).queryByRole("group", { name: "fmt" })).toBeNull();
    await expect(canvas.getByRole("button", { name: "Save" })).toBeDisabled();
  },
};

/** Fleet refused a result that would not load. Nothing was written, and the edit is still on screen. */
export const AFormEditRefused: Story = {
  name: "A refused save on the forms",
  args: { ...TheForms.args, edit: "refused" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const checks = await canvas.findByRole("region", { name: "Checks" });
    const build = within(checks).getByRole("group", { name: "build" });
    const command = within(build).getByLabelText("Command");
    await userEvent.type(command, " --offline");

    await userEvent.click(canvas.getByRole("button", { name: "Save" }));
    await expect(await canvas.findByText("Not saved")).toBeVisible();
    await expect(canvas.getByText("checks.typecheck.requires")).toBeVisible();
    await expect(command).toHaveValue("cargo build --workspace --locked --offline");
    await expect(canvas.getByRole("button", { name: "Save" })).toBeEnabled();
  },
};

/** A cost cap below what the costliest past Job here cost — a warning, never a refusal. */
export const TheBudgetWarning: Story = {
  name: "The budget warning",
  args: { ...TheForms.args, spend: { jobs: 41, most_cost_micros: 7_120_000, most_turns: 212 } },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const budget = await canvas.findByRole("region", { name: "Budget" });
    await expect(within(budget).queryByText(/~\$7\.12/)).toBeNull();
    await userEvent.type(within(budget).getByLabelText("Cost cap per Job, in dollars"), "5");
    await expect(within(budget).getByText(/~\$7\.12/)).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Save" })).toBeEnabled();
  },
};
