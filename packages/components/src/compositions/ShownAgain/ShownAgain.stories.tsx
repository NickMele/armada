import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fireEvent, fn } from "storybook/test";

import { ShownAgain } from "./ShownAgain";

/**
 * Asking a Job to show its work again, and every set a press kept.
 *
 * The frames are inline SVG data URIs, for `FramesShown`'s reason: enough to
 * prove the sets and the line above each, and small enough to read here. What a
 * real one carries is whatever the repository's own harness wrote.
 */
const meta: Meta<typeof ShownAgain> = {
  title: "Compositions/Shown again",
  component: ShownAgain,
  args: { onShow: fn(), sets: [] },
};
export default meta;

type Story = StoryObj<typeof ShownAgain>;

/**
 * A stand-in screenshot. **Named colours, and not design values** — what is in
 * a frame is somebody else's app, which no token of this system describes.
 */
function shot(fill: string, said: string): string {
  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" width="640" height="400">` +
    `<rect width="640" height="400" fill="${fill}"/>` +
    `<text x="24" y="48" fill="gainsboro" font-family="monospace" font-size="22">` +
    `${said}</text>` +
    `</svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

const SPEC = "e2e/panel.spec.ts";

/**
 * **The press can run, and says what it will run.** A person about to press is
 * shown the Drone's own spec, because that is the thing being rerun and the
 * one fact that says whether it is the right thing to look at.
 */
export const Ready: Story = {
  args: { offer: { state: "ready", spec: SPEC } },
  play: async ({ args, canvas, userEvent }) => {
    await expect(canvas.getByText(SPEC)).toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Show again" }));
    await expect(args.onShow).toHaveBeenCalledTimes(1);
  },
};

/**
 * **A press is out, and a second one cannot be sent.** The control stays where
 * it was and says what it is running, so the wait is visible and a second
 * press has nothing to land on.
 */
export const Showing: Story = {
  args: { offer: { state: "showing", spec: SPEC } },
  play: async ({ args, canvas }) => {
    const control = canvas.getByRole("button", { name: "Showing…" });
    await expect(control).toBeDisabled();
    // Dispatched rather than clicked. The app's base styles take a disabled
    // control out of pointer reach, so a pointer cannot press it at all; the
    // event still arrives here to prove the handler is not bound either.
    fireEvent.click(control);
    await expect(args.onShow).not.toHaveBeenCalled();
  },
};

/**
 * **It cannot run, and says what is missing.** A disabled control with nothing
 * beside it asks a person to guess; this is the sentence the caller read off
 * Fleet's facts, drawn where the spec would have been.
 */
export const Cannot: Story = {
  args: {
    offer: {
      state: "cannot",
      why: "This Job's worktree is gone, so there is nowhere to run the harness. A clean or a reclaim took it.",
    },
  },
  play: async ({ args, canvas }) => {
    const control = canvas.getByRole("button", { name: "Show again" });
    await expect(control).toBeDisabled();
    await expect(canvas.getByText(/worktree is gone/)).toBeInTheDocument();
    // Dispatched rather than clicked. The app's base styles take a disabled
    // control out of pointer reach, so a pointer cannot press it at all; the
    // event still arrives here to prove the handler is not bound either.
    fireEvent.click(control);
    await expect(args.onShow).not.toHaveBeenCalled();
  },
};

/**
 * **The last press ran and captured nothing.** Not a refusal — the harness ran
 * — so it is said beside the control that was pressed rather than as a failure
 * somewhere else on the screen.
 */
export const CapturedNothing: Story = {
  args: {
    offer: { state: "ready", spec: SPEC },
    said: "`evidence.run` exited 0 and `evidence.frames` held no file, so the spec captured nothing.",
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("status")).toHaveTextContent("captured nothing");
  },
};

/**
 * **Two presses are two sets, each under the moment it ran.** The owner's
 * decision on #603: a press adds a set and never replaces the step's frames or
 * an earlier press's. This is the Shown chapter's second half, drawn below the
 * step's own frames.
 */
export const TwoSets: Story = {
  args: {
    offer: { state: "ready", spec: SPEC },
    sets: [
      {
        key: "1",
        heading: "Shown again at 14:02",
        frames: [
          {
            kept: "show.again1.1.branch/panel.png",
            name: "panel.png",
            attempt: 1,
            weight: "41.0 KB",
            content: { kind: "image", src: shot("darkslategray", "Panel — 14:02") },
          },
        ],
      },
      {
        key: "2",
        heading: "Shown again at 14:19",
        frames: [
          {
            kept: "show.again2.1.branch/panel.png",
            name: "panel.png",
            attempt: 1,
            weight: "40.6 KB",
            content: { kind: "image", src: shot("midnightblue", "Panel — 14:19") },
          },
        ],
      },
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("region", { name: "Shown again at 14:02" })).toBeInTheDocument();
    await expect(canvas.getByRole("region", { name: "Shown again at 14:19" })).toBeInTheDocument();
    await expect(canvas.getAllByRole("img")).toHaveLength(2);
  },
};
