import type { Meta, StoryObj } from "@storybook/react-vite";
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
    summary: "live · 47 entries · every line opens",
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
      },
    ],
  },
};
