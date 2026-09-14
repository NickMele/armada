import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";

import type { ClaimedBreakage } from "@armada/protocol";

import type { JobFixture } from "../../../fixtures/fixture";
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

const BROKEN = "settings::selectors::visible_manifests_memoises";

/** `running`, with claimed breakages on its detail — #1001. */
function withBreakages(breakages: (jobId: string) => ClaimedBreakage[]): JobFixture {
  const fixture = running();
  if (fixture.watched.state !== "read") return fixture;
  const { detail } = fixture.watched;
  return {
    ...fixture,
    watched: { ...fixture.watched, detail: { ...detail, breakages: breakages(fixture.job.id) } },
  };
}

/** Its Check failed on a test another Job is fixing: the row names the fix. */
export const WaitingOnAFix: Story = {
  name: "Waiting on another Job's fix",
  // The rows are in Where things are, which a person opens.
  render: () => (
    <JobDetailFrom
      on={{ whereOpen: true }}
      fixture={withBreakages(() => [
        {
          check: "test",
          test: BROKEN,
          failure: "expected the same reference on repeat calls",
          fix: "01M1FIXJOB000000000000000000",
          fix_title: "Fix the selectors test broken on main",
          reported_by: "01M1REPORTER0000000000000000",
        },
      ])}
    />
  ),
  play: async ({ canvas }) => {
    await expect(await canvas.findByText(BROKEN)).toBeVisible();
    await expect(await canvas.findByText(/Fix the selectors test broken on main is fixing it/)).toBeVisible();
  },
};

/** This Job is the fix, and two Jobs wait on it: a count, never the list. */
export const FixingWithJobsWaiting: Story = {
  name: "Fixing a test two Jobs wait on",
  render: () => (
    <JobDetailFrom
      on={{ whereOpen: true }}
      fixture={withBreakages((jobId) => [
        {
          check: "test",
          test: BROKEN,
          failure: "expected the same reference on repeat calls",
          fix: jobId,
          fix_title: "Fix the selectors test broken on main",
          reported_by: "01M1REPORTER0000000000000000",
          waiting: [
            { job_id: "01M1WAITINGONE00000000000000", title: "Split the settings reducer" },
            { job_id: "01M1WAITINGTWO00000000000000", title: "Memoise the manifest list" },
          ],
        },
      ])}
    />
  ),
  play: async ({ canvas }) => {
    await expect(await canvas.findByText(/2 Jobs wait on it/)).toBeVisible();
  },
};
