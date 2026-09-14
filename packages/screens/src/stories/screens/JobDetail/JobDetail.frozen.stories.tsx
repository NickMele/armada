import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, waitFor, within } from "storybook/test";

import type { JobSummary } from "@armada/protocol";
import { queued, reviewAtDelivery } from "../../../fixtures/build/index";
import { watchedRead } from "../../../fixtures/build/base";
import type { JobFixture } from "../../../fixtures/fixture";
import { takenNotice, type Taken } from "../../../freeze";
import { TakenNotice } from "../../../Taken";
import { JobDetailFrom } from "./JobDetail";
import { drawing } from "./story-helpers";

/** Job detail, split by group — #1044. Same `title` as the rest of this directory, so ids hold. */
const meta: Meta<typeof JobDetailFrom> = {
  title: "Screens/Job detail",
  component: JobDetailFrom,
  parameters: { layout: "fullscreen" },
};
export default meta;

type Story = StoryObj<typeof JobDetailFrom>;

/** A fixture with its row, and the detail's copy of it, changed alike. */
function withRow(fixture: JobFixture, over: Partial<JobSummary>): JobFixture {
  const job = { ...fixture.job, ...over };
  if (fixture.watched.state !== "read") return { ...fixture, job };
  return { ...fixture, job, watched: watchedRead({ ...fixture.watched.detail, job }) };
}

/** Approved, and held because the repository it would land in is frozen. */
export const FrozenQueued: Story = {
  name: "Frozen, queued",
  render: drawing(() => withRow(queued(), { queued_reason: "frozen", frozen_by: ["armada"] })),
  play: async ({ canvas, canvasElement }) => {
    await expect(await canvas.findByText("Frozen")).toBeVisible();
    await waitFor(() => expect(canvasElement.textContent).toMatch(/Waits for\s*armada\s*to unfreeze/));
  },
};

/** At review in a frozen repository. The status is review's own, and the header says nothing lands. */
export const FrozenAtReview: Story = {
  name: "Frozen, at review",
  render: drawing(() => withRow(reviewAtDelivery(), { frozen_by: ["armada"] })),
  play: async ({ canvasElement }) => {
    await waitFor(() => expect(canvasElement.textContent).toMatch(/Nothing lands until\s*armada\s*unfreezes/));
  },
};

/** Merge pressed and confirmed while frozen: taken, waiting, and never drawn as a refusal. */
function MergeTakenWhileFrozenDrawn() {
  const fixture = withRow(reviewAtDelivery(), { frozen_by: ["armada"] });
  const [taken, setTaken] = useState<Taken | null>(null);
  const notice = takenNotice(taken, fixture.job);
  return (
    <JobDetailFrom
      fixture={fixture}
      above={notice === null ? null : <TakenNotice {...notice} onDismiss={() => setTaken(null)} />}
      on={{ onMergePullRequest: (jobId) => setTaken({ jobId, act: "merge", from: fixture.job.status }) }}
    />
  );
}

export const MergeTakenWhileFrozen: Story = {
  name: "Merge taken while frozen",
  render: () => <MergeTakenWhileFrozenDrawn />,
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: /^Merge/ }));
    const layer = within(await canvas.findByRole("dialog"));
    await userEvent.click(layer.getByRole("button", { name: "Merge and take the work" }));
    await expect(await canvas.findByText("Merge taken")).toBeVisible();
    await expect(canvas.getByText(/merges when the freeze lifts/)).toBeVisible();
  },
};
