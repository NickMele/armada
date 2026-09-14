import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";

import { running } from "../../../fixtures/build/index";
import { recorded } from "../../../fixtures/recorded";
import { JobDetailFrom } from "./JobDetail";
import { drawing } from "./story-helpers";

/**
 * Job detail, split by group across this directory — #1044. Every file here
 * shares this `title`, so a story's id still comes from that plus its own
 * export name and nothing that names one by id needs to change.
 */
const meta: Meta<typeof JobDetailFrom> = {
  title: "Screens/Job detail",
  component: JobDetailFrom,
  parameters: { layout: "fullscreen" },
};
export default meta;

type Story = StoryObj<typeof JobDetailFrom>;

/** A real Job that landed. Its worktree was reclaimed, so there is no diff to read. */
export const DoneRecorded: Story = {
  name: "Done (recorded)",
  render: drawing(() => recorded("done-worktree-given-back")),
};

/** The recorded Job at the narrowest window. The title and the job id stack, and the acts are one control. */
export const DoneRecordedNarrow: Story = {
  name: "Done (recorded), narrow window",
  render: () => (
    <JobDetailFrom fixture={recorded("done-worktree-given-back")} width="var(--window-floor)" />
  ),
};

/** The header's one control with its menu open: the act that leads on its face, the rest behind it. */
export const HeaderActionsOpen: Story = {
  name: "Header actions open",
  render: drawing(() => recorded("done-worktree-given-back")),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(
      await canvas.findByRole("button", { name: "Everything else this job can do" }),
    );
    await expect(await within(document.body).findByRole("menuitem", { name: /record/i })).toBeVisible();
  },
};

/** Midway through Fix, before its Check has run. */
export const Running: Story = { name: "Running", render: drawing(running) };
