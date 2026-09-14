import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, within } from "storybook/test";

import { DockQuestions } from "@armada/components";
import { runningWaitingOnACommand } from "../../../fixtures/build/index";
import { repository, watchedRead } from "../../../fixtures/build/base";
import type { JobFixture } from "../../../fixtures/fixture";
import { dockQuestionsOf } from "../../../dock-questions";
import type { Outstanding } from "../../../outstanding";
import { JobDetailFrom } from "./JobDetail";

/** Job detail, split by group — #1044. Same `title` as the rest of this directory, so ids hold. */
const meta: Meta<typeof JobDetailFrom> = {
  title: "Screens/Job detail",
  component: JobDetailFrom,
  parameters: { layout: "fullscreen" },
};
export default meta;

type Story = StoryObj<typeof JobDetailFrom>;

// #937: the dock and this screen's own question band read the same outstanding question, so an
// answer taken on either clears both. Proved here, beside `arrivals.test.ts`'s proof that the
// event which clears the dock's copy is the same one that re-reads this screen — a person's own
// window is the account the issue was closed against, and `arrivals.test.ts` cannot show a window.

/** The Job's held command, read straight off the fixture rather than written out a second time —
 * that is the whole of what "the same outstanding question" means for this story. */
function commandOutstanding(fixture: JobFixture): Outstanding {
  const whole = fixture.watched.state === "read" ? fixture.watched.detail : undefined;
  const waiting = whole?.command_waiting;
  if (waiting === undefined) throw new Error("fixture carries no command_waiting to share");
  return { kind: "command", job_id: fixture.job.id, waiting };
}

const REPOSITORIES = [repository()];

/**
 * The band and the dock, side by side, sharing one `open` flag. Whichever side answers flips it,
 * and both sides are drawn off it — never two copies of whether the question still stands.
 */
function DockAgreesWithDetailDrawn({ answerFrom }: { answerFrom: "dock" | "detail" }) {
  const base = runningWaitingOnACommand();
  const outstanding = commandOutstanding(base);
  const [open, setOpen] = useState(true);
  const whole = base.watched.state === "read" ? base.watched.detail : undefined;
  const fixture: JobFixture =
    open || whole === undefined
      ? base
      : { ...base, watched: watchedRead({ ...whole, command_waiting: undefined }) };
  const cards = dockQuestionsOf(open ? [outstanding] : [], [fixture.job], REPOSITORIES, 0, {
    onAnswer: () => setOpen(false),
  });

  return (
    <div style={{ display: "flex", height: "100vh", width: "100%" }}>
      <div style={{ flex: 1, minWidth: 0 }}>
        <JobDetailFrom
          fixture={fixture}
          on={answerFrom === "detail" ? { onAnswerCommand: () => setOpen(false) } : {}}
        />
      </div>
      <div style={{ width: "var(--w-dock)", background: "var(--bg-sunken)", padding: "var(--space-4)" }}>
        <DockQuestions questions={cards} />
      </div>
    </div>
  );
}

/** Pressing an answer in the dock's card clears the band on Job detail beside it. */
export const AnsweredInTheDockClearsJobDetail: Story = {
  name: "Answered in the dock, Job detail agrees",
  render: () => <DockAgreesWithDetailDrawn answerFrom="dock" />,
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getByRole("region", { name: "A question from the drone" })).toBeVisible();
    const card = canvas.getByRole("article");
    await userEvent.click(within(card).getByRole("button", { name: "Reject" }));
    await expect(canvas.queryByRole("article")).toBeNull();
    await expect(canvas.queryByRole("region", { name: "A question from the drone" })).toBeNull();
  },
};

/** Pressing an answer in Job detail's own band clears the dock's card beside it. */
export const AnsweredOnJobDetailClearsTheDock: Story = {
  name: "Answered on Job detail, the dock agrees",
  render: () => <DockAgreesWithDetailDrawn answerFrom="detail" />,
  play: async ({ canvas, userEvent }) => {
    const band = within(canvas.getByRole("region", { name: "A question from the drone" }));
    await userEvent.click(band.getByRole("radio", { name: "Reject" }));
    await userEvent.click(band.getByRole("button", { name: "Send this answer" }));
    await expect(canvas.queryByRole("region", { name: "A question from the drone" })).toBeNull();
    await expect(canvas.queryByRole("article")).toBeNull();
  },
};

/** A refusal clears neither. Nothing here writes local state until an answer is known to have
 * taken, so a card Fleet sent back stands beside a band that never moved. */
export const AnswerRefusedStandsOnBoth: Story = {
  name: "A refused answer stands on both",
  render: () => {
    const base = runningWaitingOnACommand();
    const cards = dockQuestionsOf([commandOutstanding(base)], [base.job], REPOSITORIES, 0, {
      onAnswer: () => {},
      refusalFor: () => "Fleet is not connected. Nothing was sent.",
    });
    return (
      <div style={{ display: "flex", height: "100vh", width: "100%" }}>
        <div style={{ flex: 1, minWidth: 0 }}>
          <JobDetailFrom fixture={base} />
        </div>
        <div style={{ width: "var(--w-dock)", background: "var(--bg-sunken)", padding: "var(--space-4)" }}>
          <DockQuestions questions={cards} />
        </div>
      </div>
    );
  },
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(within(canvas.getByRole("article")).getByRole("button", { name: "Reject" }));
    await expect(canvas.getByRole("alert")).toHaveTextContent("Fleet is not connected. Nothing was sent.");
    await expect(canvas.getByRole("region", { name: "A question from the drone" })).toBeVisible();
    await expect(canvas.getByRole("article")).toBeVisible();
  },
};
