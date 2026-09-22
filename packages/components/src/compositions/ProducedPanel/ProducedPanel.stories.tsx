import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";

import { Button } from "../../primitives/Button/Button";
import { Kbd } from "../../primitives/Kbd/Kbd";
import { ChangeSummary, ProducedGroups, ProducedPanel, type ProducedFolder } from "./ProducedPanel";

/**
 * What the Job has changed, as its own panel beside the run — #1187, drawn from
 * the #796 Job's Implement step. The two stories the issue names: a running
 * Job, and a finished one.
 */
const meta = {
  title: "Compositions/Produced panel",
  component: ProducedPanel,
  parameters: { layout: "padded" },
} satisfies Meta<typeof ProducedPanel>;

export default meta;
type Story = StoryObj<typeof meta>;

const DRAWN: ProducedFolder[] = [
  {
    path: "crates/store/src",
    added: 96,
    files: [
      { path: "crates/store/src/pending_evidence.rs", name: "pending_evidence.rs", change: "added", added: 82 },
      { path: "crates/store/src/migrations.rs", name: "migrations.rs", change: "modified", added: 12 },
      { path: "crates/store/src/lib.rs", name: "lib.rs", change: "modified", added: 2 },
    ],
  },
  {
    path: "crates/fleet/src",
    added: 58,
    deleted: 23,
    files: [
      { path: "crates/fleet/src/evidence.rs", name: "evidence.rs", change: "modified", added: 31, deleted: 9 },
      { path: "crates/fleet/src/settling.rs", name: "settling.rs", change: "modified", added: 14, deleted: 6 },
      { path: "crates/fleet/src/daemon/fittings.rs", name: "daemon/fittings.rs", change: "modified", added: 6, deleted: 1 },
      { path: "crates/fleet/src/dispatch.rs", name: "dispatch.rs", change: "modified", added: 4, deleted: 4 },
      { path: "crates/fleet/src/ending.rs", name: "ending.rs", change: "modified", added: 3, deleted: 3 },
    ],
  },
];

const OPEN_THE_DIFF = (
  <Button variant="ghost" size="sm">
    Open the diff
    <Kbd>f</Kbd>
  </Button>
);

/**
 * A running Job. The newest file has arrived since Fleet last counted, so it is
 * listed with no numbers and drawn last, rather than as the smallest change.
 */
export const Running: Story = {
  name: "A running Job",
  args: {
    summary: "9 files · +154 −23",
    act: OPEN_THE_DIFF,
    children: (
      <ChangeSummary
        emptyNote="No changes yet"
        folders={[
          DRAWN[0] as ProducedFolder,
          {
            ...(DRAWN[1] as ProducedFolder),
            files: [
              ...(DRAWN[1] as ProducedFolder).files,
              { path: "crates/fleet/src/reloading.rs", name: "reloading.rs", change: "added" },
            ],
          },
        ]}
      />
    ),
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("button", { name: /Open the diff/ })).toBeVisible();
    const store = canvas.getByRole("region", { name: "crates/store/src" });
    await expect(store).toHaveTextContent("pending_evidence.rsnew+82");
    await expect(canvas.getByRole("region", { name: "crates/fleet/src" })).toHaveTextContent(/reloading\.rsnew$/);
  },
};

/** A finished Job whose worktree was given back: the count taken as it stopped, a long list cut, and no patch to open. */
export const Finished: Story = {
  name: "A finished Job",
  args: {
    summary: "8 files · +154 −23 · all inside the plan",
    children: (
      <ChangeSummary
        emptyNote="Nothing was written"
        folders={[
          { ...(DRAWN[0] as ProducedFolder), files: (DRAWN[0] as ProducedFolder).files.slice(0, 1) },
          { ...(DRAWN[1] as ProducedFolder), files: (DRAWN[1] as ProducedFolder).files.slice(0, 1) },
        ]}
        more="and 6 more files, in the diff"
      />
    ),
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText("and 6 more files, in the diff")).toBeVisible();
    // Nothing to open once the worktree is given back.
    await expect(canvas.queryByRole("button", { name: /Open the diff/ })).toBeNull();
  },
};

/**
 * The groups a landed Job ran (#1542), inside the same panel: what each came
 * to, its tasks, what it wrote and the commit it left.
 *
 * **`not timed` is in the column and not a dash.** The record times a step, and
 * a group sits between a step and a task, so there is no instant to subtract —
 * which is a fact about what Fleet keeps rather than a group that took no time.
 */
export const Groups: Story = {
  args: {
    summary: "4 groups · 8 tasks · 9 files",
    children: (
      <ProducedGroups
        emptyNote="This Job recorded no plan, so it ran as one piece."
        note="Nothing times a group: the record times a step, so no group here carries a span of its own."
        groups={[
          { name: "Group one", verb: "passed", status: "completed-success", tasks: "2 of 2 done", files: "3 files", checks: "4 Checks", commit: "4c1b9d2" },
          { name: "Group two", verb: "passed", status: "completed-success", tasks: "2 of 2 done", files: "3 files", checks: "7 Checks", commit: "7a2f0c5" },
          { name: "Group three", verb: "passed", status: "completed-success", tasks: "2 of 2 done", files: "2 files", checks: "7 Checks, twice", commit: "b81c3e4" },
          { name: "Group four", verb: "landed", status: "completed-success", tasks: "2 of 2 done", files: "3 files", checks: "7 Checks", commit: "e0d47a1" },
        ]}
      />
    ),
  },
};

/** A Job whose plan nothing recorded. The list says so rather than drawing nothing. */
export const NoGroups: Story = {
  args: {
    summary: "no plan",
    children: (
      <ProducedGroups
        groups={[]}
        emptyNote="This Job recorded no plan, so it ran as one piece."
      />
    ),
  },
};
