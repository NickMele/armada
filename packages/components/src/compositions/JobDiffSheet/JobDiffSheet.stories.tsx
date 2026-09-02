import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";

import { JobDiffSheet, railOfPatch, type JobDiffFile } from "./JobDiffSheet";
import { UnifiedDiff, type DiffFile } from "../UnifiedDiff/UnifiedDiff";

/**
 * The Job's patch on a trailing sheet — Journey 4, frame `4j`.
 *
 * The sheet is laid out inside the nearest positioned ancestor, so the story
 * draws one: outside a screen there is nothing for it to be flush to.
 */
const meta: Meta<typeof JobDiffSheet> = {
  title: "Compositions/Job diff sheet",
  component: JobDiffSheet,
  decorators: [
    (Story) => (
      <div
        style={{
          position: "relative",
          height: "var(--palette-max-height)",
          background: "var(--bg-base)",
        }}
      >
        <Story />
      </div>
    ),
  ],
};
export default meta;

type Story = StoryObj<typeof JobDiffSheet>;

const PATCH: DiffFile[] = [
  {
    path: "packages/settings/src/selectors.ts",
    lines: [
      { kind: "hunk", text: "@@ -14,6 +14,9 @@ import { createSelector } from 'reselect'" },
      { kind: "context", text: " import type { SettingsState } from './types'" },
      { kind: "added", text: "+import { selectColumnOrder } from './selectors/columns'" },
      { kind: "context", text: " export const selectSettings = (s: RootState) => s.settings" },
      {
        kind: "removed",
        text: "-export const selectColumns = createSelector([selectSettings], (s) => s.columns)",
      },
      {
        kind: "added",
        text: "+export const selectColumns = createSelector([selectSettings], (s) => s.columns)",
      },
    ],
  },
  {
    path: "packages/settings/src/reducer.ts",
    lines: [
      { kind: "hunk", text: "@@ -48,7 +51,7 @@ export const selectDensity = …" },
      { kind: "removed", text: "-  return state.settings.density ?? 'comfortable'" },
      { kind: "added", text: "+  return state.settings.density ?? DEFAULT_DENSITY" },
    ],
  },
  {
    path: "packages/settings/test/useColumnSelectors.test.ts",
    lines: [
      { kind: "hunk", text: "@@ -0,0 +1,21 @@" },
      { kind: "added", text: "+import { selectVisibleColumns } from '../src/selectors'" },
    ],
  },
];

/** What one Job's rail says nothing about. Absent everywhere Bridge draws it. */
const WROTE_IT: Record<string, string> = {
  "packages/settings/src/selectors.ts": "Fix",
  "packages/settings/src/reducer.ts": "Fix",
  "packages/settings/test/useColumnSelectors.test.ts": "Reproduction",
};

/**
 * The rail, counted off the patch, with the one thing a patch cannot say added.
 *
 * **The counts are not written here.** They were, and they disagreed with the
 * patch beside them by an order of magnitude — a registry entry modelling the
 * shape #310 turned out to be. `railOfPatch` is what Bridge calls, so what the
 * story draws is what the app draws.
 */
const RAIL: JobDiffFile[] = railOfPatch(PATCH).map((file) => ({
  ...file,
  step: WROTE_IT[file.path],
}));

/** Opened from a step's `Produced`, at that step's first file. */
export const OpenedAtAStep: Story = {
  args: {
    open: true,
    branch: "fix/settings-split-selectors",
    files: RAIL,
    selected: PATCH[0]!.path,
    openedAt: "Fix",
    children: (
      <UnifiedDiff files={PATCH} emptyNote="This drone has not changed anything yet." />
    ),
  },
};

/** At `--window-floor`: flush to both edges, a narrower rail, an icon close. */
export const AtTheFloor: Story = {
  args: { ...OpenedAtAStep.args, floor: true },
};

/**
 * A Job mid-step: a patch from the worktree and no footprint behind it — #310.
 *
 * **This is the state the sheet is opened in most.** Somebody forty minutes
 * into an Implement step wants to know which files the drone has touched, and
 * nothing has submitted, so there is no read-back to take a file list from. The
 * rail is counted off the patch instead, which means the header cannot say
 * `0 files` while the body draws three of them.
 *
 * No `openedAt` and no `step` on any row: the reading is the whole Job's
 * worktree and nothing served says which step wrote a file. The note under the
 * rail says so, over a list rather than over nothing.
 */
export const MidStepWithNoFootprint: Story = {
  args: {
    open: true,
    branch: "fix/settings-split-selectors",
    files: railOfPatch(PATCH),
    note:
      "Fleet commits once at the end, so the patch is the Job's. Nothing served says which " +
      "step wrote each file.",
    children: (
      <UnifiedDiff files={PATCH} emptyNote="This drone has not changed anything yet." />
    ),
  },
  play: async ({ canvas }) => {
    const sheet = canvas.getByRole("dialog", { name: "Job diff" });

    // The whole of the defect: a count of nothing over a patch full of files.
    await expect(sheet).not.toHaveTextContent("0 files");
    await expect(sheet).toHaveTextContent("3 files");

    // Every file in the patch, on the rail, and the header's totals are the
    // sum of them rather than a second reading's.
    for (const file of PATCH) {
      await expect(canvas.getByRole("button", { name: new RegExp(file.path) })).toBeVisible();
    }
    await expect(sheet).toHaveTextContent("+4");
    await expect(sheet).toHaveTextContent("−2");

    // The note is still there, and it now reads as what it says rather than as
    // an excuse for an empty rail.
    await expect(sheet).toHaveTextContent("Nothing served says which step wrote each file.");
  },
};

/**
 * Nothing was read — the third header state, and the reason it exists.
 *
 * **A count of nothing is a claim, and here it would be a false one.** A Job
 * with no worktree, a read Fleet did not answer and a read still in flight all
 * arrive as no reading, and `0 files · +0 −0` over any of them says the drone
 * changed nothing. That is #310 one state along, so the header says it has no
 * reading and leaves the explanation to the body.
 *
 * `files: []` is a different story and not this one: a worktree that opened and
 * holds no change genuinely is `0 files`, and it says so.
 */
export const NoReading: Story = {
  args: {
    open: true,
    branch: "fix/settings-split-selectors",
    files: null,
    children: (
      <UnifiedDiff
        files={[]}
        emptyNote={
          "This job has no worktree, so there is nothing to read. Absent is not empty — a " +
          "drone that changed nothing is a different answer, and this is not it."
        }
      />
    ),
  },
  play: async ({ canvas }) => {
    const sheet = canvas.getByRole("dialog", { name: "Job diff" });

    // The header states the absence. It never asserts a count, and it drops the
    // clause that says where a patch came from, because there is no patch.
    await expect(sheet).toHaveTextContent("no reading");
    await expect(sheet).not.toHaveTextContent("0 files");
    await expect(sheet).not.toHaveTextContent("+0");
    await expect(sheet).not.toHaveTextContent("uncommitted, in the worktree");

    // Which silence it is comes from the body, not the header.
    await expect(sheet).toHaveTextContent("This job has no worktree");

    // No rail rows to select, and the note is still under them.
    await expect(canvas.queryAllByRole("button", { name: /packages\// })).toHaveLength(0);
    await expect(sheet).toHaveTextContent("Fleet commits once at the end");
  },
};
