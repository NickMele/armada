import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";

import { FramesShown } from "./FramesShown";

/**
 * What a step whose point is not the code is reviewed by.
 *
 * The frames here are inline SVG data URIs rather than real screenshots —
 * enough to prove the plate, the fit and the provenance line, and small enough
 * to read in the file. What a real one carries is a PNG a repository's own
 * harness wrote.
 */
const meta: Meta<typeof FramesShown> = {
  title: "Compositions/Frames shown",
  component: FramesShown,
};
export default meta;

type Story = StoryObj<typeof FramesShown>;

/**
 * A stand-in screenshot, at a screen's shape.
 *
 * **Named colours, and they are not design values.** What is inside a frame is
 * a repository's own screenshot: Armada never authored it, and no token of this
 * design system applies to a picture of somebody else's app. A hex here would
 * read as a colour somebody chose from the palette — which is what the design
 * rule refuses, and exactly what this is not.
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

/**
 * The ordinary reading: two frames from one run, each with the name its spec
 * gave it.
 *
 * **The names are the whole of what a reader scans**, which is why they are
 * mono and why nothing here paraphrases them. A harness that names its frames
 * well has said something; one that names them `1.png` has not, and this draws
 * that honestly rather than inventing a caption for it.
 */
export const TwoFrames: Story = {
  args: {
    frames: [
      {
        kept: "show.1/job-detail-refused.png",
        name: "job-detail-refused.png",
        attempt: 1,
        weight: "41.0 KB",
        src: shot("darkslategray", "Job detail — refused"),
      },
      {
        kept: "show.1/job-detail-advanced.png",
        name: "job-detail-advanced.png",
        attempt: 1,
        weight: "38.4 KB",
        src: shot("midnightblue", "Job detail — advanced"),
      },
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.getAllByRole("img")).toHaveLength(2);
    // The provenance is on every frame, never implied by grouping.
    await expect(canvas.getByText("attempt 1 · 41.0 KB")).toBeInTheDocument();
  },
};

/**
 * **Three runs, and every frame says which one it came from.**
 *
 * A step worked three times captured three sets and they are three different
 * screens. Nothing groups them and nothing needs to: the run is on the row, so
 * a reader looking at the frame that mattered is never counting down a list to
 * work out whether it is the current one.
 */
export const EveryRunSaysWhichItIs: Story = {
  args: {
    frames: [1, 2, 3].map((attempt) => ({
      kept: `show.${attempt}/home.png`,
      name: "home.png",
      attempt,
      weight: "40.2 KB",
      src: shot(attempt === 3 ? "midnightblue" : "maroon", `attempt ${attempt}`),
    })),
  },
  play: async ({ canvas }) => {
    // The same file name three times — which is exactly why the run is on the
    // row, and why `kept` and not `name` is what identifies one.
    await expect(canvas.getAllByText("home.png")).toHaveLength(3);
    await expect(canvas.getByText("attempt 3 · 40.2 KB")).toBeInTheDocument();
  },
};

/**
 * **A frame still being read holds its own space.**
 *
 * The plate is sized from an aspect ratio rather than from the image, so three
 * frames landing one at a time do not walk the chapter down the screen. The
 * word is `reading…` and not a blank: a blank plate is indistinguishable from a
 * frame of a blank page, which is the one thing this surface must never be
 * mistaken for.
 */
export const StillReading: Story = {
  args: {
    frames: [
      {
        kept: "show.1/home.png",
        name: "home.png",
        attempt: 1,
        weight: "412 KB",
        src: shot("darkslategray", "home"),
      },
      { kept: "show.1/settings.png", name: "settings.png", attempt: 1, weight: "388 KB" },
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("reading…")).toBeInTheDocument();
    // The weight is there before the bytes are, which is what makes a slow one
    // legible as a large file rather than as a broken read.
    await expect(canvas.getByText("attempt 1 · 388 KB")).toBeInTheDocument();
  },
};

/**
 * **A frame that would not come back says why, where the reason is its own.**
 *
 * A reclaimed `.armada/frames` is the case this exists for: the row is on the
 * record and the file is not. That is a sentence about this frame and not a
 * state for the step, so it goes on the plate rather than replacing the list.
 */
export const OneCouldNotBeRead: Story = {
  args: {
    frames: [
      {
        kept: "show.1/home.png",
        name: "home.png",
        attempt: 1,
        weight: "40.2 KB",
        src: shot("darkslategray", "home"),
      },
      {
        kept: "show.1/settings.png",
        name: "settings.png",
        attempt: 1,
        weight: "38.8 KB",
        why: "this frame is on the record and no longer on disk",
      },
    ],
  },
  play: async ({ canvas }) => {
    await expect(
      canvas.getByText("this frame is on the record and no longer on disk"),
    ).toBeInTheDocument();
    // The one that did come back is still drawn. A failed read is one frame's
    // business, never the list's.
    await expect(canvas.getAllByRole("img")).toHaveLength(1);
  },
};

/**
 * Openable. A frame at panel width is a thumbnail of a screen, and a screen
 * shrunk to 602px is evidence of nothing — so the plate is a control wherever a
 * caller can put the whole thing somewhere bigger.
 */
export const Openable: Story = {
  args: {
    ...TwoFrames.args,
    onOpen: () => {},
  },
  play: async ({ canvas }) => {
    await expect(
      canvas.getByRole("button", { name: "Open job-detail-refused.png" }),
    ).toBeInTheDocument();
  },
};

/**
 * **No handler, no control.** A record being read rather than a surface being
 * acted on — the rule the file rail follows one composition over. The images
 * are still images; what is gone is the button around them.
 */
export const ReadOnly: Story = {
  args: TwoFrames.args,
  play: async ({ canvas }) => {
    await expect(canvas.queryAllByRole("button")).toHaveLength(0);
  },
};

/**
 * A step with no frames. **The ordinary case, and not a failure** — most steps
 * declare no `visual` evidence at all, so which silence this is belongs to the
 * caller and this draws what it is given.
 */
export const NoneAtAll: Story = {
  args: {
    frames: [],
    emptyNote: "This step declares no visual evidence, so its harness never ran.",
  },
};

/**
 * **The reading #209 was written for: what changed, not what is.**
 *
 * The harness ran twice — the base checkout serving with the branch's own spec
 * shooting, then the branch serving and shooting — and each plate says which
 * side it is. `home.png` is on both, so it is a before and an after.
 * `settings.png` is on the branch only, which is a screen the change *added*
 * and has no before; drawing that as a gap would make the commonest case here
 * read as the feature being broken.
 *
 * **`before` and `after`, not `base` and `branch`.** The wire's words name the
 * two checkouts Fleet had to serve; these are the ones a reviewer thinks in,
 * and the caller does the translation once.
 */
export const BeforeAndAfter: Story = {
  args: {
    frames: [
      {
        kept: "show.1.base/home.png",
        name: "home.png",
        attempt: 1,
        side: "before",
        weight: "37.2 KB",
        src: shot("dimgray", "Home — as it was"),
      },
      {
        kept: "show.1.branch/home.png",
        name: "home.png",
        attempt: 1,
        side: "after",
        weight: "41.0 KB",
        src: shot("darkslategray", "Home — with the change"),
      },
      {
        kept: "show.1.branch/settings.png",
        name: "settings.png",
        attempt: 1,
        side: "after",
        weight: "38.8 KB",
        src: shot("darkslateblue", "Settings — new screen"),
      },
    ],
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("attempt 1 · before · 37.2 KB")).toBeInTheDocument();
    // The added screen still says which side it is, so a reader counting halves
    // can see that this one has none.
    await expect(canvas.getByText("attempt 1 · after · 38.8 KB")).toBeInTheDocument();
  },
};
