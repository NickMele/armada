import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";

import { StudioFrameSheet } from "./StudioFrameSheet";

/**
 * A Note's frame at the size a person reads it, over the Studios surface it was
 * opened from.
 */
const meta: Meta<typeof StudioFrameSheet> = {
  title: "Compositions/Studio frame sheet",
  component: StudioFrameSheet,
};
export default meta;

type Story = StoryObj<typeof StudioFrameSheet>;

/** A stand-in screenshot. A named colour: a photograph of a window is nobody's design value. */
function shot(fill: string, said: string): string {
  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" width="1440" height="900">` +
    `<rect width="1440" height="900" fill="${fill}"/>` +
    `<text x="48" y="96" fill="gainsboro" font-family="monospace" font-size="42">` +
    `${said}</text>` +
    `</svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

const SAID = "The legend under the step bar is unreadable at this width";

export const Opened: Story = {
  args: {
    open: true,
    said: SAID,
    frame: { src: shot("darkslategray", "Job Board — 1440 × 900") },
  },
  play: async ({ canvas }) => {
    // What the person said names the layer; nothing captions the picture.
    await expect(canvas.getByRole("dialog", { name: "Note" })).toBeVisible();
    await expect(canvas.getByText(SAID)).toBeVisible();
    await expect(canvas.getByRole("img", { name: /captured from/ })).toBeVisible();
  },
};

/** The Note is still a Note when its picture cannot be read, and the sheet says which. */
export const NothingToDraw: Story = {
  args: {
    open: true,
    said: SAID,
    frame: { why: "This frame is on the Note and no longer on disk." },
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText(/no longer on disk/)).toBeVisible();
    await expect(canvas.queryByRole("img")).toBeNull();
  },
};
