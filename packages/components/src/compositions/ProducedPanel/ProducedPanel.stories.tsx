import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect } from "storybook/test";

import { Button } from "../../primitives/Button/Button";
import { Kbd } from "../../primitives/Kbd/Kbd";
import { ChangeSummary, ProducedPanel, type ProducedFolder } from "./ProducedPanel";

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
