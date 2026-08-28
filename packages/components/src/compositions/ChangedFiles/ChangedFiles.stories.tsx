import type { Meta, StoryObj } from "@storybook/react-vite";
import { ChangedFiles } from "./ChangedFiles";

/**
 * What a Drone has changed in its worktree, while it is still working.
 *
 * A Drone that is working and a Drone that is thrashing look identical from the
 * outside; a file list is the cheapest thing that tells them apart. Names and a
 * change kind only — the diff is the expensive read and it is a later question.
 */
const meta: Meta<typeof ChangedFiles> = {
  title: "Compositions/Changed files",
  component: ChangedFiles,
};
export default meta;

type Story = StoryObj<typeof ChangedFiles>;

const NOTHING_YET = "This drone has not changed anything yet.";

/** A drone part way through a step, on a step that declared no plan. */
export const WhatADroneHasTouched: Story = {
  args: {
    emptyNote: NOTHING_YET,
    note: "Read from the worktree while the drone was working. This step declared no plan, so no row is marked.",
    files: [
      { path: "crates/api/src/routes.rs", change: "modified" },
      { path: "crates/ipc/src/history.rs", change: "added" },
      { path: "crates/ipc/src/lib.rs", change: "modified" },
      { path: "crates/ipc/src/legacy_events.rs", change: "deleted" },
    ],
  },
};

/**
 * A step that declared a plan, with two paths outside it. The mark is #88's
 * already-computed drift restated, never a new judgement: drift does not fail a
 * step, and the drone answers it by declaring again.
 */
export const TwoPathsOutsideThePlan: Story = {
  args: {
    emptyNote: NOTHING_YET,
    note: "Read from the worktree while the drone was working. 2 of 5 paths are outside the plan this step declared.",
    files: [
      { path: "apps/desktop/src/renderer/src/JobDetail.tsx", change: "modified" },
      { path: "apps/desktop/src/shared/protocol.ts", change: "modified" },
      { path: "packages/tokens/src/status.css", change: "modified", outsidePlan: true },
      { path: "packages/components/src/compositions/ChangedFiles/ChangedFiles.tsx", change: "added" },
      { path: "scripts/dev", change: "type_changed", outsidePlan: true },
    ],
  },
};

/**
 * The kinds that are not an edit. Each renders the wire's own word — nothing
 * folds `conflicted` or `unreadable` into "modified", because those are the two
 * a person has to act on.
 */
export const TheKindsThatAreNotAnEdit: Story = {
  args: {
    emptyNote: NOTHING_YET,
    files: [
      { path: "docs/scope.md", change: "renamed" },
      { path: "docs/journeys/watch-a-drone.md", change: "copied" },
      { path: "crates/fleet/src/serving.rs", change: "conflicted" },
      { path: "assets/AppIcon.icns", change: "unreadable" },
    ],
  },
};

/** A drone that has written nothing so far. Ordinary, and never an error. */
export const NothingChangedYet: Story = {
  args: { files: [], emptyNote: NOTHING_YET },
};
