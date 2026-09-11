import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
import { ActivityLog } from "../ActivityLog/ActivityLog";
import { ChangedFiles } from "../ChangedFiles/ChangedFiles";
import { FramesShown } from "../FramesShown/FramesShown";
import { StepStory, type StepChapter } from "./StepStory";

/**
 * The story as the drawing tells it, and what happens when a chapter is
 * opened. Press `Open the log` in the first story: the log grows in place and
 * the two chapters around it collapse to their header lines, so the order
 * stays on screen while one part of it is long.
 */
const meta: Meta<typeof StepStory> = {
  title: "Compositions/Step story",
  component: StepStory,
};
export default meta;

type Story = StoryObj<typeof StepStory>;

const PREVIEW = [
  { id: "1", at: "14:22:07", actor: "armada" as const, summary: "Go on to Implement." },
  { id: "2", at: "14:26:31", actor: "drone" as const, summary: "Edit", subject: "packages/settings/src/selectors.ts" },
  {
    id: "3",
    at: "14:29:40",
    actor: "drone" as const,
    summary: "Bash",
    subject: "cargo build --workspace --locked",
    output: "$ cargo build --workspace --locked\n    Finished `dev` profile [unoptimized] in 47.61s",
    ran: "exit 0 · 47.61s · in .armada/worktrees/job_2d90bb",
  },
  { id: "4", at: "14:30:28", actor: "fleet" as const, summary: "Heartbeat — the Drone has been quiet for 48 seconds" },
  { id: "5", at: "14:31:58", actor: "drone" as const, summary: "thinking" },
];

const WHOLE = [
  ...PREVIEW.slice(0, 1),
  { id: "1b", at: "14:22:44", actor: "drone" as const, summary: "Splitting the selector block into its own module so the tests can import it without the store." },
  { id: "1c", at: "14:23:11", actor: "drone" as const, summary: "Read", subject: "packages/settings/src/reducer.ts" },
  ...PREVIEW.slice(1),
];

const FILES = (
  <ChangedFiles
    emptyNote="This drone has not changed anything yet."
    files={[
      { path: "packages/settings/src/selectors.ts", change: "modified" },
      { path: "packages/settings/src/reducer.ts", change: "modified" },
      { path: "packages/settings/src/index.ts", change: "added" },
    ]}
  />
);

const CHAPTERS: StepChapter[] = [
  {
    id: "instructions",
    ordinal: 1,
    title: "Drone instructions",
    summary: "14:22:07",
    preview:
      "Move the selector block into its own module so the tests can import it without constructing " +
      "the store. Do not change reducer behaviour.",
  },
  {
    id: "log",
    ordinal: 2,
    title: "Activity log",
    live: true,
    summary: "47 entries",
    preview: <ActivityLog entries={PREVIEW} />,
    content: <ActivityLog entries={WHOLE} />,
    openLabel: "Open the log — all 47 entries",
  },
  {
    id: "produced",
    ordinal: 3,
    title: "Produced",
    summary: "3 files · +94 −31 · all inside the plan",
    preview: FILES,
    content: FILES,
    openLabel: "Open the diff — 3 files",
  },
];

/** The story at rest: three chapters, each showing what it holds. */
export const TheStory: Story = {
  args: { chapters: CHAPTERS },
  /**
   * **A header folds its own chapter and no other.** Folding the log leaves
   * Produced open, and the log opens again on the same press.
   *
   * The body is hidden rather than unmounted: the log is still streaming into a
   * chapter that is shut, and the header's `aria-controls` still has to name
   * something.
   */
  play: async ({ canvas, userEvent }) => {
    const log = canvas.getByRole("button", { name: /Activity log/ });
    const produced = canvas.getByRole("button", { name: /Produced/ });
    await expect(log).toHaveAttribute("aria-expanded", "true");
    await expect(produced).toHaveAttribute("aria-expanded", "true");

    await userEvent.click(log);
    await expect(log).toHaveAttribute("aria-expanded", "false");
    await expect(produced).toHaveAttribute("aria-expanded", "true");

    const body = document.getElementById(log.getAttribute("aria-controls") ?? "");
    await expect(body).not.toBeNull();
    await expect(body).not.toBeVisible();

    await userEvent.click(log);
    await expect(log).toHaveAttribute("aria-expanded", "true");
  },
};

/**
 * The log open. The chapters around it stay as they are, which
 * still say what each holds — the story's order is intact while one part of it
 * is long.
 */
export const TheLogOpen: Story = {
  args: { chapters: CHAPTERS, openId: "log" },
};

/**
 * The diff open. **Opening the diff closes the log.** That is the answer to
 * the height problem and the constraint that makes this different from a stack
 * of accordions — and its cost is that you cannot read the transcript beside
 * the diff, which is how a Drone narrating one thing and doing another is
 * caught. If that matters, the fix is a split for those two only.
 */
export const TheDiffOpen: Story = {
  args: { chapters: CHAPTERS, openId: "produced" },
};

/**
 * A fourth chapter, on a step waiting for a person: the decision sits at the
 * end rather than in the header, because you make it after reading. The acts
 * that interrupt a Drone stay in the panel header; this one concludes.
 */
export const WithADecision: Story = {
  args: {
    chapters: [
      ...CHAPTERS,
      {
        id: "decision",
        ordinal: 4,
        title: "Your decision",
        summary: "nothing advances until you answer",
        preview: "Approve, send back with a note, or reject. Send back returns it to this step; reject ends the Job.",
        tone: "waiting",
      },
    ],
  },
};

/**
 * Held by the caller. `openChapter` is the whole of what is open and pressing a
 * header only reports — **this story is deliberately inert**, because that is
 * what a controlled component does when nobody holds the other end.
 *
 * It exists for a keyboard map that has to open chapter two by name. The
 * alternative it replaces is a caller reaching into the DOM for
 * `.armada-story__chapter` and clicking the first button it finds, which works
 * until this component renames a class.
 */
export const HeldByTheCaller: Story = {
  args: { chapters: CHAPTERS, openChapter: "log", onOpen: fn() },
  /**
   * The inertness the prose above calls deliberate, asserted — because inert
   * and broken are the same drawing. The log's whole content is open because
   * the caller says so, and pressing its Close asks the caller rather than
   * closing it here. Folding by a header is the reader's and never reaches the
   * caller.
   */
  play: async ({ args, canvas, userEvent }) => {
    const produced = canvas.getByRole("button", { name: /Produced/ });
    await expect(produced).toHaveAttribute("aria-expanded", "true");

    await userEvent.click(canvas.getByRole("button", { name: /^Close/ }));
    await expect(args.onOpen).toHaveBeenCalledWith(null);
    await expect(canvas.getByRole("button", { name: /^Close/ })).toBeVisible();

    await userEvent.click(produced);
    await expect(args.onOpen).toHaveBeenCalledTimes(1);
  },
};

/**
 * **A chapter with only a preview still folds**, and gains no control at the
 * foot of its body, because there is nothing past the preview. Whether the
 * header can be pressed and whether there is more to show are two questions.
 */
export const AChapterWithOnlyAPreviewStillFolds: Story = {
  args: { chapters: CHAPTERS },
  play: async ({ canvas, userEvent }) => {
    const instructions = canvas.getByRole("button", { name: /Drone instructions/ });
    const log = canvas.getByRole("button", { name: /Activity log/ });

    await userEvent.click(instructions);
    await expect(instructions).toHaveAttribute("aria-expanded", "false");
    await expect(log).toHaveAttribute("aria-expanded", "true");

    await userEvent.click(instructions);
    await expect(instructions).toHaveAttribute("aria-expanded", "true");
    await expect(canvas.queryByRole("button", { name: "Close" })).toBeNull();
  },
};

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

/**
 * **The story leads with what it looks like, and the diff is one level down.**
 *
 * The inverse of every story above, and the one arrangement the panel departs
 * for. On a change whose point is not the code — a panel that should collapse,
 * a screen that should render differently — the diff is the least useful thing
 * on screen and it was the only thing offered: reviewing meant reading a patch
 * to infer an outcome you could have been shown. `#209`.
 *
 * **The frames are in the preview, not behind an act.** Opening a chapter to
 * find out that there are pictures is the gesture this removes. Every other
 * long chapter here opens; this one is already open by being drawn.
 *
 * **The numbers move with it.** Produced reads `4` here and `3` in every story
 * above. An ordinal is the position in the story a reader navigates by, so it
 * is counted over what was actually drawn — a fixed number per chapter was
 * right until a chapter could appear before Produced.
 *
 * **No caption under any frame, and no field for one.** A screenshot of the
 * wrong state looks exactly like one of the right state; what makes a frame
 * checkable is the spec that produced it, which is code in the diff beside the
 * change. What each frame carries instead is the name its spec chose, the run
 * it came from, and what it weighs.
 */
export const ShowingTheOutcome: Story = {
  args: {
    chapters: [
      { ...CHAPTERS[0]!, ordinal: 1 },
      { ...CHAPTERS[1]!, ordinal: 2 },
      {
        id: "shown",
        ordinal: 3,
        title: "Shown",
        says: "Click to see what the step produced",
        // Two runs, because that is the case a summary has to distinguish: two
        // frames from one run and two from two are different things to be
        // looking at.
        summary: "2 frames · 2 runs",
        preview: (
          <FramesShown
            frames={[
              {
                kept: "show.1/job-detail-refused.png",
                name: "job-detail-refused.png",
                attempt: 1,
                weight: "41.0 KB",
                content: { kind: "image", src: shot("darkslategray", "attempt 1 — still wrong") },
              },
              {
                kept: "show.2/job-detail-refused.png",
                name: "job-detail-refused.png",
                attempt: 2,
                weight: "40.2 KB",
                content: { kind: "image", src: shot("midnightblue", "attempt 2 — collapsed") },
              },
            ]}
          />
        ),
      },
      { ...CHAPTERS[2]!, ordinal: 4 },
    ],
  },
  play: async ({ canvasElement }) => {
    // **The order is the claim**, so it is read off the story in one go.
    const named = (selector: string) =>
      Array.from(canvasElement.querySelectorAll(selector)).map((at) => at.textContent);

    expect(named(".armada-chapter__name")).toEqual([
      "Drone instructions",
      "Activity log",
      "Shown",
      "Produced",
    ]);
    expect(named(".armada-chapter__n")).toEqual(["1", "2", "3", "4"]);

    // On screen without anything being pressed. That is the whole of leading
    // with the evidence.
    expect(canvasElement.querySelectorAll(".armada-frames__image")).toHaveLength(2);
  },
};
