import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";

import { FramesPaired } from "./FramesPaired";

/**
 * What the change did to the screen.
 *
 * The pictures are inline SVG rather than real screenshots — enough to prove
 * the plate, the flip and the fold, and small enough to read in the file. What
 * a real one carries is a PNG a repository's own harness wrote.
 */
const meta: Meta<typeof FramesPaired> = {
  title: "Compositions/Frames paired",
  component: FramesPaired,
};
export default meta;

type Story = StoryObj<typeof FramesPaired>;

/**
 * A stand-in screenshot, at a screen's shape.
 *
 * **Named colours, and they are not design values.** What is inside a frame is
 * a repository's own screenshot: Armada never authored it, and no token of this
 * design system applies to a picture of somebody else's app.
 */
function shot(fill: string, said: string): string {
  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" width="640" height="400">` +
    `<rect width="640" height="400" fill="${fill}"/>` +
    `<text x="24" y="52" fill="gainsboro" font-family="monospace" font-size="24">` +
    `${said}</text>` +
    `</svg>`;
  return `data:image/svg+xml;utf8,${encodeURIComponent(svg)}`;
}

function side(name: string, kept: string, said: string, fill: string) {
  return {
    kept,
    name,
    attempt: 1,
    weight: "40.2 KB",
    content: { kind: "image" as const, src: shot(fill, said) },
  };
}

/**
 * **The reading this surface exists for: ten screens, one of them moved.**
 *
 * Drawing all twenty would ask a person to do the comparison the panel is
 * supposed to do for them. The three that did not move are one line each — kept
 * rather than dropped, because a screen that stayed put is what says the change
 * stayed where it was asked to.
 */
export const OneOfFourMoved: Story = {
  args: {
    pairs: [
      { name: "home.png", attempt: 1, same: true, before: side("home.png", "b1", "home", "darkslategray"), after: side("home.png", "a1", "home", "darkslategray") },
      { name: "settings.png", attempt: 1, same: true, before: side("settings.png", "b2", "settings", "darkslategray"), after: side("settings.png", "a2", "settings", "darkslategray") },
      {
        name: "checks-panel.png",
        attempt: 1,
        same: false,
        before: side("checks-panel.png", "b3", "4 rows", "maroon"),
        after: side("checks-panel.png", "a3", "1 line", "midnightblue"),
      },
      { name: "job-detail.png", attempt: 1, same: true, before: side("job-detail.png", "b4", "detail", "darkslategray"), after: side("job-detail.png", "a4", "detail", "darkslategray") },
    ],
  },
  play: async ({ canvas }) => {
    // One plate drawn, three folded to a line. Asserted through roles rather
    // than text: a folded pair keeps its body in the document so that
    // `aria-controls` names something real, and only the role queries respect
    // `hidden`.
    await expect(canvas.getAllByRole("img")).toHaveLength(1);
    await expect(canvas.getAllByText("unchanged")).toHaveLength(3);
    // The one that moved is the one that is pressable, and it opens on the
    // after — so the control offers the before.
    await expect(
      canvas.getByRole("button", { name: "Show the before of checks-panel.png" }),
    ).toBeVisible();
  },
};

/**
 * **A folded pair opens.** Folded rather than dropped: a reader who wants to
 * check that a screen really did stay put can, which is `RunTree`'s rule for a
 * spent attempt one surface over — the outcome stays, the working folds.
 */
export const AFoldedPairOpens: Story = {
  args: OneOfFourMoved.args,
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getAllByRole("img")).toHaveLength(1);
    await userEvent.click(canvas.getAllByRole("button", { name: /home\.png/ })[0]!);
    await expect(canvas.getAllByRole("img")).toHaveLength(2);
  },
};

/**
 * **Hold to compare.** The plate opens on the after and shows the before while
 * the pointer is on it. A difference nobody can find by comparing two images is
 * obvious the moment one replaces the other — the changed region is the only
 * thing that moves.
 */
export const HoldToCompare: Story = {
  args: {
    pairs: [
      {
        name: "checks-panel.png",
        attempt: 1,
        same: false,
        before: side("checks-panel.png", "b", "4 rows", "maroon"),
        after: side("checks-panel.png", "a", "1 line", "midnightblue"),
      },
    ],
  },
  play: async ({ canvas, userEvent }) => {
    const plate = canvas.getByRole("button", { name: /Show the before/ });
    await expect(canvas.getByText("after")).toBeVisible();
    await userEvent.hover(plate);
    await expect(canvas.getByText("before")).toBeVisible();
    await userEvent.unhover(plate);
    await expect(canvas.getByText("after")).toBeVisible();
  },
};

/**
 * **A screen the change added has no before, and that is the answer.**
 *
 * The commonest case `#209` exists for is a brand-new screen, where there was
 * never anything to compare against. The plate is not a control — there is
 * nothing to flip to — and the caption says which of the two silences this is
 * rather than leaving a reader to wonder what failed.
 */
export const AddedAndRemoved: Story = {
  args: {
    pairs: [
      {
        name: "cleared-tab.png",
        attempt: 1,
        same: false,
        after: side("cleared-tab.png", "a", "new screen", "midnightblue"),
      },
      {
        name: "old-banner.png",
        attempt: 1,
        same: false,
        before: side("old-banner.png", "b", "was here", "maroon"),
      },
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("nothing was here before")).toBeVisible();
    await expect(canvas.getByText("this screen is gone")).toBeVisible();
    // Neither is pressable: a control that flips to nothing is a dead click.
    await expect(canvas.queryAllByRole("button")).toHaveLength(0);
  },
};

/**
 * **Nothing moved at all**, which is a real answer and not an empty state. The
 * change touched no screen the spec photographs — worth saying plainly, because
 * on a `shown` step it is the one outcome that asks a question.
 */
export const NothingMoved: Story = {
  args: {
    pairs: [
      { name: "home.png", attempt: 1, same: true, before: side("home.png", "b1", "home", "darkslategray"), after: side("home.png", "a1", "home", "darkslategray") },
      { name: "settings.png", attempt: 1, same: true, before: side("settings.png", "b2", "settings", "darkslategray"), after: side("settings.png", "a2", "settings", "darkslategray") },
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.queryAllByRole("img")).toHaveLength(0);
    await expect(canvas.getAllByText("unchanged")).toHaveLength(2);
  },
};

/**
 * A pair still being read holds its own space. The plate is sized from an
 * aspect ratio, so frames landing one at a time do not walk the chapter down
 * the screen — and the word is `reading…` rather than a blank, which would be
 * indistinguishable from a photograph of a blank page.
 */
export const StillReading: Story = {
  args: {
    pairs: [
      {
        name: "checks-panel.png",
        attempt: 1,
        same: false,
        before: { kept: "b", name: "checks-panel.png", attempt: 1, weight: "41.0 KB" },
        after: side("checks-panel.png", "a", "1 line", "midnightblue"),
      },
    ],
  },
  play: async ({ canvas, userEvent }) => {
    await userEvent.hover(canvas.getByRole("button", { name: /Show the before/ }));
    await expect(canvas.getByText("reading…")).toBeVisible();
  },
};

/** A step with no frames at all. The ordinary case, and not a failure. */
export const NoneAtAll: Story = {
  args: {
    pairs: [],
    emptyNote: "This step's harness ran and captured nothing to look at.",
  },
};
