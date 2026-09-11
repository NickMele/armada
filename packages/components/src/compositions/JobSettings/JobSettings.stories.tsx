import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { JobSettings, JobSettingsButton, type JobSettingsChoice } from "./JobSettings";

/**
 * Every setting a person can change on a running Job, on the layer the log,
 * the patch and Pulse already use. Opened from the Job header's `Job settings`.
 *
 * The sheet is laid out inside the nearest positioned ancestor, so every story
 * draws one: outside a screen there is nothing for it to be flush to. It is a
 * window high, as the run sheet's is, so the whole panel is read without
 * scrolling the layer.
 */
const meta: Meta<typeof JobSettings> = {
  title: "Compositions/Job settings",
  component: JobSettings,
  args: {
    open: true,
    jobId: "77-split-the-settings-reducer",
    onModel: fn(),
    onWhenBlocked: fn(),
    onRemove: fn(),
    onClose: fn(),
  },
  decorators: [
    (Story) => (
      <div
        style={{
          position: "relative",
          height: "100vh",
          background: "var(--bg-base)",
        }}
      >
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof JobSettings>;

/** The three answers, in the words `copy.ts` gives them. */
const CHOICES: JobSettingsChoice[] = [
  {
    value: "refuse_and_hold",
    label: "Stop and wait for me",
    means: "It's refused, and the job stops until you allow or reject it.",
  },
  {
    value: "ask_me",
    label: "Ask me first",
    means: "The drone waits while you decide, then carries on. The job shows under Needs you.",
  },
  {
    value: "allow_all",
    label: "Run it",
    means:
      "Any command runs without asking, so the job finishes however it can. Pushing stays " +
      "Armada's, and commands armada.yml marks destructive still stop for you.",
  },
];

const MODELS = ["haiku", "sonnet", "opus"];

const COST = { cap: "$60.00", used: "~$29.63", onRaise: fn() };
const TURNS = { cap: "1000", used: "580", onRaise: fn() };

/**
 * **A Job exactly as it started.** Stop and wait for me, the workflow's model,
 * nothing allowed — the defaults every Job is dispatched with.
 */
export const AtRest: Story = {
  name: "At rest",
  args: {
    costCap: COST,
    turnCap: TURNS,
    models: MODELS,
    model: null,
    choices: CHOICES,
    whenBlocked: "refuse_and_hold",
    allowed: [],
  },
  /**
   * **What goes is the wire's word, never the label.** `Run it` is copy and
   * Fleet takes `allow_all`; a model goes by the name `list_models` gave it.
   * Broken once by handing `choice.label` to the callback, and once by
   * swapping the select's two answers so a named model went as `null`.
   */
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("radio", { name: "Run it" }));
    await expect(args.onWhenBlocked).toHaveBeenCalledWith("allow_all");

    await userEvent.selectOptions(canvas.getByLabelText("Model for the next step"), "opus");
    await expect(args.onModel).toHaveBeenCalledWith("opus");
  },
};

/**
 * **Run it chosen, and the list kept.** Nothing needs allowing while every
 * command runs, so the list dims rather than empties — switching back finds it
 * where it was. The line under the choice is what a change says once it took.
 */
export const RunItChosen: Story = {
  name: "Run it chosen, list dimmed",
  args: {
    costCap: COST,
    turnCap: TURNS,
    models: MODELS,
    model: null,
    choices: CHOICES,
    whenBlocked: "allow_all",
    whenBlockedSaid: "Changed. Applies the next time it reaches for a command.",
    allowed: [
      { run: "pnpm add -D reselect@5.1.1", reach: "job" },
      { run: "pnpm exec vitest run packages/settings", reach: "job" },
    ],
  },
};

/**
 * **An allow that also wrote armada.yml.** The row says so, and what Remove
 * takes away is this Job's allow only — the line in the file outlives it. A
 * model is chosen here, so handing the choice back is the other half to press.
 */
export const ARepositoryRow: Story = {
  name: "A repository row",
  args: {
    costCap: COST,
    turnCap: TURNS,
    models: MODELS,
    model: "opus",
    modelSaid: "Changed. The next step starts on opus.",
    choices: CHOICES,
    whenBlocked: "ask_me",
    allowed: [
      { run: "pnpm add -D reselect@5.1.1", reach: "job" },
      { run: "cargo xtask verify-foundations", reach: "repository" },
    ],
  },
  /**
   * **Remove names its own row, and the first option is a clear.** A Remove
   * handed the wrong row's command takes back an allow nobody pressed on; the
   * workflow's choice is the empty option and has to go as `null`, which is the
   * one value Fleet reads as handing the choice back. Broken once by sending
   * the first row's command from every Remove, and once by swapping the
   * select's two answers so the workflow's choice went as an empty name.
   */
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(
      canvas.getByRole("button", { name: "Remove cargo xtask verify-foundations" }),
    );
    await expect(args.onRemove).toHaveBeenCalledWith("cargo xtask verify-foundations");

    await userEvent.selectOptions(
      canvas.getByLabelText("Model for the next step"),
      "The workflow's choice",
    );
    await expect(args.onModel).toHaveBeenCalledWith(null);
  },
};

/**
 * **Every control off**, the reading not live or a change already out. The
 * raise buttons go with the rest, and one sentence says why rather than a panel
 * of controls that silently do nothing.
 */
export const ControlsOff: Story = {
  name: "Controls off",
  args: {
    costCap: COST,
    turnCap: TURNS,
    models: MODELS,
    model: null,
    choices: CHOICES,
    whenBlocked: "refuse_and_hold",
    allowed: [{ run: "pnpm add -D reselect@5.1.1", reach: "job" }],
    disabled: true,
    disabledNote: "This job is not live, so nothing can be changed.",
  },
};

/**
 * The header's way in. **Quiet at rest, and a count once anything differs** from
 * how a Job starts — here a model and one allowed command.
 */
export const TheHeaderButton: StoryObj<typeof JobSettingsButton> = {
  name: "The header button",
  render: () => (
    <div style={{ display: "flex", gap: "var(--space-2)", padding: "var(--space-4)" }}>
      <JobSettingsButton changed={0} onOpen={fn()} />
      <JobSettingsButton changed={2} onOpen={fn()} />
    </div>
  ),
};
