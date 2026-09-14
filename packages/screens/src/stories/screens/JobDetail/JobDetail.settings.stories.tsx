import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import { running, runningWaitingOnACommand } from "../../../fixtures/build/index";
import { JOB_ID, watchedRead } from "../../../fixtures/build/base";
import { WAITING_CALL } from "../../../fixtures/build/running";
import type { JobFixture } from "../../../fixtures/fixture";
import { JobDetailFrom } from "./JobDetail";

/** Job detail, split by group — #1044. Same `title` as the rest of this directory, so ids hold. */
const meta: Meta<typeof JobDetailFrom> = {
  title: "Screens/Job detail",
  component: JobDetailFrom,
  parameters: { layout: "fullscreen" },
};
export default meta;

type Story = StoryObj<typeof JobDetailFrom>;

/** What the panel's choice sends. A module's spy, cleared by the play that reads it. */
const setWhenBlocked = fn();

/**
 * The running Job, changed: a model chosen for its later steps and one command
 * allowed for it. What the header counts, and what the panel lists.
 */
function runningWithSettings(): JobFixture {
  const fixture = running();
  if (fixture.watched.state !== "read") return fixture;
  return {
    ...fixture,
    watched: watchedRead({
      ...fixture.watched.detail,
      model_override: "opus",
      allowed_commands: [
        {
          run: "pnpm add -D reselect@5.1.1",
          reach: "job",
          allowed_at: "2026-09-10T14:29:40Z",
          by: "human",
        },
      ],
      // Fleet's own table, since protocol 13.5 — covers every job against
      // this repository, so the panel draws it read-only. #836's own case:
      // `gh issue view`, always-allowed from Job 7 while filing #834.
      repository_allowed_commands: [
        {
          run: "gh issue view",
          reach: "repository",
          allowed_at: "2026-09-13T09:41:00Z",
          by: "human",
        },
      ],
    }),
  };
}

/**
 * **The header's way in, and the panel it opens.** The count on the button is
 * the model and the allow; the panel says what each setting does and when a
 * change to it takes. Nothing on the screen behind it restarts.
 *
 * **What goes is this Job's id and the wire's word.** The panel's own story
 * proves a choice names `ask_me`; this proves the screen hands it on against the
 * Job on screen, which is the half a composition cannot show. Broken once by
 * sending the Job's handle rather than its id.
 */
export const JobSettingsOpen: Story = {
  name: "Job settings open",
  render: () => (
    <JobDetailFrom fixture={runningWithSettings()} on={{ onSetWhenBlocked: setWhenBlocked }} />
  ),
  play: async ({ canvas, userEvent }) => {
    setWhenBlocked.mockClear();
    await userEvent.click(await canvas.findByRole("button", { name: /^Job settings/ }));
    const panel = within(await canvas.findByRole("dialog", { name: "Job settings" }));
    await userEvent.click(panel.getByRole("radio", { name: "Ask me first" }));
    await expect(setWhenBlocked).toHaveBeenCalledWith(JOB_ID, "ask_me");

    // The repository-wide row: read-only, and it carries no Remove — that
    // reaches every job against this repository, which is the Manifest
    // screen's act rather than this job's own settings.
    await expect(panel.getByText("gh issue view")).toBeVisible();
    await expect(panel.queryByRole("button", { name: "Remove gh issue view" })).toBeNull();
  },
};

/** What a waiting command's answer sends. A module's spy, cleared by the play that reads it. */
const answerCommand = fn();

/**
 * A Job at Ask me first whose Drone reached for a command it was not given, and
 * is waiting on a person — still `running`.
 * The command is what is asked; the answers are the three Fleet offered, each
 * saying what it commits to.
 *
 * **What goes is the call and the answer's wire name**, never the words on
 * the control. A label is copy, and Fleet takes `allow_for_job`.
 */
export const WaitingOnACommand: Story = {
  name: "Waiting on a command",
  render: () => (
    <JobDetailFrom fixture={runningWaitingOnACommand()} on={{ onAnswerCommand: answerCommand }} />
  ),
  play: async ({ canvas, userEvent }) => {
    answerCommand.mockClear();
    await userEvent.click(await canvas.findByRole("radio", { name: "Allow for this job" }));
    await userEvent.click(canvas.getByRole("button", { name: "Send this answer" }));
    // No note: an allow tells the drone everything it acts on, and the field is
    // never drawn under it.
    await expect(answerCommand).toHaveBeenCalledWith(
      JOB_ID,
      WAITING_CALL,
      "allow_for_job",
      undefined,
      undefined,
    );
  },
};

/**
 * The same command, always allowed — with the rule Fleet suggested, cut short
 * of the version pin.
 *
 * **What goes is the rule picked, never the whole command.** `pnpm` is the
 * candidate a person did not choose, and `pnpm add` is what Fleet declares.
 */
export const AlwaysAllowingWithARule: Story = {
  name: "Always allowing, with a rule",
  render: () => (
    <JobDetailFrom fixture={runningWaitingOnACommand()} on={{ onAnswerCommand: answerCommand }} />
  ),
  play: async ({ canvas, userEvent }) => {
    answerCommand.mockClear();
    await userEvent.click(
      await canvas.findByRole("radio", { name: "Always allow in this repository" }),
    );
    await expect(canvas.getByRole("radio", { name: "pnpm add" })).toBeChecked();

    await userEvent.click(canvas.getByRole("button", { name: "Send this answer" }));
    await expect(answerCommand).toHaveBeenCalledWith(
      JOB_ID,
      WAITING_CALL,
      "always_allow",
      undefined,
      "pnpm add",
    );
  },
};

/**
 * The same command, refused with a reason.
 *
 * **What goes is the call, the wire's word and the person's own words** — one
 * act, because the reason exists at the moment somebody presses reject and a
 * second surface for it is a second chance to lose it.
 */
export const RejectingWithAReason: Story = {
  name: "Rejecting a command, with a reason",
  render: () => (
    <JobDetailFrom fixture={runningWaitingOnACommand()} on={{ onAnswerCommand: answerCommand }} />
  ),
  play: async ({ canvas, userEvent }) => {
    answerCommand.mockClear();
    await userEvent.click(await canvas.findByRole("radio", { name: "Reject" }));
    await userEvent.type(
      canvas.getByLabelText("Note (optional)"),
      "we are not taking that dependency",
    );
    await userEvent.click(canvas.getByRole("button", { name: "Send this answer" }));
    await expect(answerCommand).toHaveBeenCalledWith(
      JOB_ID,
      WAITING_CALL,
      "reject",
      "we are not taking that dependency",
      undefined,
    );
  },
};

/** What the reading asks for, and what comes back. A module's spy, cleared by its play. */
const explainCommand = fn(async () => ({
  ok: true as const,
  explained: {
    explanation:
      "It adds reselect 5.1.1 to this repository as a development dependency and writes the lockfile. It reaches the network.",
    model: "haiku",
  },
}));

/**
 * Reading the command before answering it.
 *
 * **It decides nothing**, which is the second half of this play: the three
 * answers are live after the reading lands, exactly as they were before anybody
 * pressed. The model is named, because a reading is a claim and not a fact.
 */
export const ReadingACommand: Story = {
  name: "Reading a command before answering",
  render: () => (
    <JobDetailFrom fixture={runningWaitingOnACommand()} on={{ onExplainCommand: explainCommand }} />
  ),
  play: async ({ canvas, userEvent }) => {
    explainCommand.mockClear();
    await userEvent.click(
      await canvas.findByRole("button", { name: "Help me understand this command" }),
    );
    await expect(explainCommand).toHaveBeenCalledWith(JOB_ID, WAITING_CALL);

    const reading = within(await canvas.findByRole("status", { name: "What this command does" }));
    await expect(reading.getByText(/adds reselect 5\.1\.1/)).toBeVisible();
    await expect(reading.getByText("haiku")).toBeVisible();
    await expect(canvas.getByRole("radio", { name: "Reject" })).toBeEnabled();
  },
};
