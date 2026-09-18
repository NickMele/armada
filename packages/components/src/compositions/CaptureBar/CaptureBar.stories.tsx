import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent } from "storybook/test";

import { ACTION } from "../../generated/actions";
import { CaptureBar } from "./CaptureBar";

/** The binding is the registry's own, as Studio capture's is. */
const BINDING = [...(ACTION.capture_note?.shortcut ?? "")];

const meta: Meta<typeof CaptureBar> = {
  title: "Compositions/Capture bar",
  component: CaptureBar,
  args: {
    run: "web_dev",
    address: "http://127.0.0.1:41207",
    studio: "The checkout flow",
    serving: true,
    armed: false,
    framesRefused: 0,
    onArm: fn(),
    onReload: fn(),
    onFollowRefused: fn(),
    binding: BINDING,
  },
};
export default meta;

type Story = StoryObj<typeof CaptureBar>;

/** Serving: the Run, the address in full, and the Studio a Note will land on. */
export const Serving: Story = {
  play: async ({ canvas }) => {
    await expect(canvas.getByText("http://127.0.0.1:41207")).toBeVisible();
    await expect(canvas.getByText("Notes land on The checkout flow")).toBeVisible();
    // No address field, no Back and no Forward: this is not a browser.
    await expect(canvas.queryByRole("textbox")).toBeNull();
  },
};

/** Armed: a press on the page points rather than acts. */
export const Capturing: Story = {
  args: { armed: true },
};

/**
 * A refused address is drawn in full and never followed. One act offers it to
 * the browser this person already uses.
 */
export const Refused: Story = {
  args: {
    refused: {
      address: "https://accounts.google.com/o/oauth2/v2/auth?client_id=1",
      said: "Navigation",
      offerable: true,
    },
  },
  play: async ({ args, canvas }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Open in browser" }));
    await expect(args.onFollowRefused).toHaveBeenCalled();
  },
};

/** A subframe off the origin is counted rather than named: a page draws many. */
export const FramesRefused: Story = {
  args: { framesRefused: 3 },
};

/**
 * The run ended. **Capture is closed and the window loads nothing further** —
 * a loopback port is not an identity, and anything may bind it once the server
 * exits.
 */
export const RunEnded: Story = {
  args: { serving: false },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: "Capture" })).toBeDisabled();
    await expect(canvas.getByRole("button", { name: "Reload" })).toBeDisabled();
  },
};
