import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { running } from "../../../fixtures/build/index";
import { JobDetailFrom } from "./JobDetail";
import { PLAN_PARTWAY, withPlan } from "../../../../../../../apps/desktop/src/renderer/src/mock/job-plan-fixtures";

/** Job detail, split by group — #1044. Same `title` as the rest of this directory, so ids hold. */
const meta: Meta<typeof JobDetailFrom> = {
  title: "Screens/Job detail",
  component: JobDetailFrom,
  parameters: { layout: "fullscreen" },
};
export default meta;

type Story = StoryObj<typeof JobDetailFrom>;

/** What each story below spies on. Module spies, cleared by the plays that read them. */
const compose = fn();

/**
 * `n` — `new_job`'s scope in `actions.toml` is `anywhere`, not `detail`, so
 * the screen answers it the same way the Board and Overview's lists do: with
 * no step, no chapter and no row under the cursor at all.
 */
export const DispatchKeyOpensTheComposer: Story = {
  name: "Dispatch key opens the composer",
  render: () => <JobDetailFrom fixture={running()} on={{ onCompose: compose }} />,
  play: async ({ userEvent }) => {
    compose.mockClear();
    await userEvent.keyboard("n");
    await expect(compose).toHaveBeenCalledTimes(1);
  },
};

/**
 * Typing "n" into a text field is typing, not a shortcut. `holdsText` in
 * `keys.ts` is the one guard every single-key press in `detailPressOf` goes
 * through, and this is the same rule one field over from the Board's search
 * box — the Drop dialog's Reason field is what this screen has open by
 * default with a plan on it.
 */
export const DispatchKeyIsSuppressedWhileTyping: Story = {
  name: "Dispatch key is suppressed while typing",
  render: () => <JobDetailFrom fixture={withPlan(PLAN_PARTWAY)} on={{ onCompose: compose }} />,
  play: async ({ canvas, userEvent }) => {
    compose.mockClear();
    const found = await canvas.findByText("Add a unit test that does not construct the store");
    const row = found.closest("li");
    if (row === null) throw new Error("the task row was not found");
    await userEvent.click(within(row).getByRole("button", { name: "Drop…" }));
    await userEvent.type(within(row).getByLabelText("Reason"), "n");
    await expect(compose).not.toHaveBeenCalled();
  },
};
