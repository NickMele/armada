import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, waitFor, within } from "storybook/test";

import { escalatedGateFailure, gateChecksStreaming, running } from "../../../fixtures/build/index";
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

/**
 * The log, opened from its chapter's own control, the way a person opens it.
 * The control leaves the chapter once its sheet is open, which is what this
 * checks.
 */
export const LogOpen: Story = {
  name: "Log open",
  render: drawing(running),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: /Open the log/ }));
    await expect(canvas.queryByRole("button", { name: /Open the log/ })).toBeNull();
  },
};

/** The Job's patch, opened from the Produced chapter. */
export const DiffOpen: Story = {
  name: "Diff open",
  render: drawing(running),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: /Open the diff/ }));
  },
};

/** The log open at the narrowest window Bridge lays out for. */
export const LogOpenNarrow: Story = {
  name: "Log open, narrow window",
  render: () => <JobDetailFrom fixture={running()} width="var(--window-floor)" />,
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: /Open the log/ }));
  },
};

/** The log open on a Job a failed Check stopped. */
export const LogOpenStopped: Story = {
  name: "Log open, stopped",
  render: drawing(escalatedGateFailure),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: /Open the log/ }));
  },
};

/** The failed Check's output, opened from the header act — the editor, not the sheet. */
export const CheckOutputOpen: Story = {
  name: "Check output open",
  render: drawing(escalatedGateFailure),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: /Open the output/ }));
  },
};

// #1021 — a Check's log had no end and was drawn under the rows anyway, which
// pushed every later chapter off the screen. Both stories below are the fix's
// own definition of done: a press, live or kept, opens the sheet; `Esc`
// returns to the Checks chapter; nothing is ever drawn inline.

/**
 * A kept Check's row, pressed. **The sheet, not the chapter, is what grows.**
 * Nothing between the rows and the next chapter gets any taller.
 */
export const CheckOutputRowOpensSheet: Story = {
  name: "Check output row opens the sheet, kept",
  render: drawing(escalatedGateFailure),
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.queryByRole("dialog")).toBeNull();
    // `aria-pressed` is `CheckRuns`' own row control, so this is the one that
    // opens the sheet rather than the run tree's gate row, which names the
    // same file for copying into a shell and carries no pressed state at all.
    await userEvent.click(
      await canvas.findByRole("button", {
        name: "regression_verify.3.cargo_nextest.log",
        pressed: false,
      }),
    );
    const body = within(document.body);
    const dialog = within(await body.findByRole("dialog", { name: "Console output" }));
    await expect(dialog.findByText(/visible_manifests_memoises/)).resolves.toBeVisible();
    // The one place the file's own name and the pressed Check agree.
    await expect(dialog.findByText("cargo_nextest — output")).resolves.toBeVisible();

    // The second exit, same as the log and diff sheets.
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(body.queryByRole("dialog")).toBeNull());
  },
};

/**
 * A running Check's row, pressed while the gate is still writing it. The
 * sheet opens on the same press and asks main to follow the log — nothing
 * about "which Check is filling the chapter" survives from before #1021,
 * because nothing fills the chapter any more.
 */
export const CheckOutputRowOpensSheetLive: Story = {
  name: "Check output row opens the sheet, live",
  render: () => (
    <JobDetailFrom fixture={gateChecksStreaming()} on={{ onFollowCheckOutput: fn() }} />
  ),
  play: async ({ canvas, userEvent }) => {
    // `aria-pressed` is `CheckRuns`' own row control — the same query the
    // kept story uses. #1045: this used to time out because the Checks phase
    // was not the timeline's default-open row while a Check streamed.
    await userEvent.click(
      await canvas.findByRole("button", {
        name: "regression_verify.1.cargo_nextest.live.log",
        pressed: false,
      }),
    );
    const body = within(document.body);
    const dialog = within(await body.findByRole("dialog", { name: "Console output" }));
    // No `followed` state was wired for this story, so main has not answered
    // yet — which is itself the proof the sheet asked, rather than drawing
    // whatever the chapter already had.
    await expect(dialog.findByText(/Opening this Check.s log/)).resolves.toBeVisible();
  },
};

/** Pulse, read in full: the Details control on its title line opens the sheet. */
export const FullReadingOpen: Story = {
  name: "Full reading open",
  render: drawing(running),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: /^Details/ }));
  },
};
