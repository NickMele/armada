import type { Meta, StoryObj } from "@storybook/react-vite";
import { JobDiffSheet, type JobDiffFile } from "./JobDiffSheet";
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

/** The rail's own reading: the counts, and the step that wrote each file. */
const RAIL: JobDiffFile[] = [
  { path: "packages/settings/src/selectors.ts", added: 61, removed: 4, step: "Fix" },
  { path: "packages/settings/src/reducer.ts", added: 12, removed: 27, step: "Fix" },
  {
    path: "packages/settings/test/useColumnSelectors.test.ts",
    added: 21,
    removed: 0,
    step: "Reproduction",
  },
];

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
