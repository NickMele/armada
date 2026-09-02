import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";
import { ActivityLog } from "../ActivityLog/ActivityLog";
import { ChangedFiles } from "../ChangedFiles/ChangedFiles";
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
    summary: "47 entries · every line opens",
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
   * **Opening one chapter closes the other.** That is the answer to the height
   * problem, and it is the constraint that makes this a story rather than a
   * stack of accordions — two long chapters open at once is the state an
   * accordion reaches on its second press and this must never reach at all.
   *
   * The last two lines are the other half of the rule: collapsed is hidden,
   * not unmounted. The log is still streaming into a chapter that is shut, and
   * the header's `aria-controls` still has to name something.
   */
  play: async ({ canvas, userEvent }) => {
    const log = canvas.getByRole("button", { name: /Activity log/ });
    const produced = canvas.getByRole("button", { name: /Produced/ });

    // Nothing open, so every chapter is showing what it holds.
    await expect(log).toHaveAttribute("aria-expanded", "true");
    await expect(produced).toHaveAttribute("aria-expanded", "true");

    await userEvent.click(log);
    await expect(log).toHaveAttribute("aria-expanded", "true");
    await expect(produced).toHaveAttribute("aria-expanded", "false");

    await userEvent.click(produced);
    await expect(produced).toHaveAttribute("aria-expanded", "true");
    await expect(log).toHaveAttribute("aria-expanded", "false");

    const body = document.getElementById(log.getAttribute("aria-controls") ?? "");
    await expect(body).not.toBeNull();
    await expect(body).not.toBeVisible();
  },
};

/**
 * The log open. The chapters around it collapse to their header lines, which
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
   * and broken are the same drawing. A component that kept its own copy of the
   * open chapter beside the caller's would look right in every story where the
   * caller writes back, and disagree with it silently everywhere else.
   */
  play: async ({ args, canvas, userEvent }) => {
    const produced = canvas.getByRole("button", { name: /Produced/ });
    await expect(produced).toHaveAttribute("aria-expanded", "false");

    await userEvent.click(produced);
    await expect(args.onOpen).toHaveBeenCalledWith("produced");
    await expect(produced).toHaveAttribute("aria-expanded", "false");
  },
};
