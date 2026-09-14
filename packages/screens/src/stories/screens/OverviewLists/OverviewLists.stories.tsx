import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, fn, within } from "storybook/test";
import type { BoardSection } from "../../../board";
import { ARMADA, RUNNING_ONE_WITH_A_PLAN, acrossTwo, OverviewListsFrom, STOREFRONT } from "./OverviewLists";

/**
 * Overview's lists, drawn by the app's own `OverviewLists`: Needs you, Running, Queued and Other,
 * over the Jobs in scope, as the Board's own rows. #920.
 */
const meta = {
  title: "Screens/Overview lists",
  component: OverviewListsFrom,
  parameters: { layout: "padded" },
} satisfies Meta<typeof OverviewListsFrom>;

export default meta;
type Story = StoryObj<typeof meta>;

/** One Job in every section, and one no tab claims — named beneath the lists, since the row shape
 * cannot draw a status this build's registry has never heard of. */
export const EverySection: Story = {
  name: "Every section",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText("Needs you")).toBeVisible();
    await expect(canvas.getByText("Running")).toBeVisible();
    await expect(canvas.getByText("Queued")).toBeVisible();
    await expect(canvas.queryByText("Done")).toBeNull();
    await expect(canvas.getByText(/not_a_status_the_registry_has/)).toBeVisible();
  },
};

/** Nothing in scope: the same null-result reading the Board itself draws. */
export const NoJobs: Story = {
  name: "No jobs",
  args: { jobs: [] },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByText("No jobs. Propose one above.")).toBeVisible();
  },
};

/** Two repositories served, on All: every row names its own, the Board's own rule. */
export const OnAllTwoRepositories: Story = {
  name: "On All, two repositories",
  args: { jobs: acrossTwo(), repositories: [ARMADA, STOREFRONT] },
  play: async ({ canvasElement }) => {
    const values = [...canvasElement.querySelectorAll(".armada-job-row__field-value")].map(
      (one) => one.textContent ?? "",
    );
    await expect(values.some((value) => /^(armada|storefront)$/.test(value))).toBe(true);
  },
};

/**
 * Fleet unreachable: what Bridge holds is what it last received, so a Needs you row from before
 * the outage stays on screen rather than the lists reading as though nothing needs anyone.
 */
export const Disconnected: Story = {
  name: "Fleet unreachable",
  args: { disconnected: "Fleet unreachable", stale: true },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByText("Needs you")).toBeVisible();
    await expect(canvasElement.querySelectorAll("[data-job-id]").length).toBeGreaterThan(0);
  },
};

/**
 * Two running rows, one with a plan and one without — the owner's own report against this panel.
 * The row is the Board's own `card` view, so its shared columns line the two up whether or not
 * either row carries a plan; screenshotted rather than measured here — geometry is what the
 * screenshot is for.
 */
export const PanelRowsAligned: Story = {
  name: "Panel rows aligned",
  args: { jobs: RUNNING_ONE_WITH_A_PLAN, repositories: [ARMADA, STOREFRONT] },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("img", { name: "0 of 6 tasks" })).toBeVisible();
    await expect(canvas.getByRole("option", { name: /unanswered permission ask/ })).toBeVisible();
    await expect(canvas.getByRole("option", { name: /shows "queued"/ })).toBeVisible();
  },
};

/**
 * Focus is the cursor, whichever panel it lands in — `Jobs.tsx`'s own rule,
 * reported up the same way. `#1075` reads this for Helm's context.
 */
export const CursorReportsTheFocusedRow: Story = {
  name: "Cursor reports the focused row",
  args: { onCursor: fn() },
  play: async ({ args, canvasElement }) => {
    const row = canvasElement.querySelector<HTMLElement>("[data-job-id]");
    if (row === null) throw new Error("a row to focus");
    row.focus();
    await expect(args.onCursor).toHaveBeenCalledWith(row.dataset.jobId);
  },
};

/**
 * Overview 27's own fold (#1091), wired through this screen rather than pressed straight on
 * `ActiveJobsList` — proves `section.id` reaches the right panel's `open`/`onOpenChange`, which
 * `ActiveJobsList`'s own story cannot see from inside one list.
 */
export const RunningFolds: Story = {
  name: "Running folds and reopens",
  render: function Render() {
    const [open, setOpen] = useState<Partial<Record<BoardSection, boolean>>>({});
    return (
      <OverviewListsFrom
        openSections={open}
        onSectionOpenChange={(section, value) => setOpen((was) => ({ ...was, [section]: value }))}
      />
    );
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("heading", { name: "Running" })).toBeVisible();
    const options = canvas.getAllByRole("option");
    await expect(options.length).toBeGreaterThan(0);

    const collapse = canvas.getByRole("button", { name: "Collapse Running" });
    await collapse.click();
    // Needs you and Queued still hold their own rows; only Running's left.
    await expect(canvas.queryByRole("button", { name: "Collapse Needs you" })).toBeVisible();

    await canvas.getByRole("button", { name: "Expand Running" }).click();
    await expect(canvas.getByRole("button", { name: "Collapse Running" })).toBeVisible();
  },
};

/** Fleet unreachable with nothing ever held: the fault Bridge cannot fix, named flatly. */
export const DisconnectedWithNothingHeld: Story = {
  name: "Fleet unreachable, nothing held",
  args: { jobs: [], disconnected: "Fleet unreachable", stale: true },
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByText("Fleet is not connected, so there is nothing to show."),
    ).toBeVisible();
  },
};
