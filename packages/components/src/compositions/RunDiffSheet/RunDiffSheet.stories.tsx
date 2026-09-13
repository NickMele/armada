import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn } from "storybook/test";

import { RunDiffSheet } from "./RunDiffSheet";
import type { DiffFile } from "../UnifiedDiff/UnifiedDiff";

/**
 * What one run in the main checkout changed — Journey 9's **Open the diff**.
 *
 * **Against the snapshot the run took, never `HEAD`.** The three states the
 * wire can answer are the three stories: a patch, a patch from a run that was
 * since undone, and a snapshot that is gone.
 *
 * The sheet is laid out inside the nearest positioned ancestor, so the story
 * draws one — on the Manifest surface that ancestor is `RunPage`.
 */
const meta: Meta<typeof RunDiffSheet> = {
  title: "Compositions/Run diff sheet",
  component: RunDiffSheet,
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

type Story = StoryObj<typeof RunDiffSheet>;

/** What `cargo fmt --all` did to two files, as git wrote the patch. */
const REFORMATTED: DiffFile[] = [
  {
    path: "crates/fleet/src/rehearsing/checkout.rs",
    lines: [
      { kind: "hunk", text: "@@ -137,9 +137,7 @@ impl Fleet {" },
      { kind: "context", text: "         let gone = |why: &str| {" },
      { kind: "removed", text: "-            answered(ipc::RunDiffReading::Gone {" },
      { kind: "removed", text: "-                why: why.to_string()," },
      { kind: "removed", text: "-            })" },
      { kind: "added", text: "+            answered(ipc::RunDiffReading::Gone { why: why.to_string() })" },
      { kind: "context", text: "         };" },
    ],
  },
  {
    path: "crates/api/src/rehearsing.rs",
    lines: [
      { kind: "hunk", text: "@@ -158,7 +158,8 @@ pub(crate) async fn get_checkout_run_diff" },
      { kind: "removed", text: "-    match served.daemon().get_checkout_run_diff(run_id).await {" },
      { kind: "added", text: "+    match served.daemon().get_checkout_run_diff(run_id).await" },
      { kind: "added", text: "+    {" },
    ],
  },
];

const RAN = { name: "fmt", ranAt: "14:18:02", onClose: fn() };

/** A run that changed the checkout: the patch, counted in the header, against its snapshot. */
export const ARunThatChangedFiles: Story = {
  args: {
    ...RAN,
    open: true,
    reading: { state: "read", files: REFORMATTED, emptyNote: "This run changed nothing." },
  },
};

/**
 * **An undone run's diff is still drawn**, under a band that says it was
 * undone. Undo restores from the snapshot and keeps it — so the patch reads,
 * and a person deciding whether to run `fmt` again can see what it did.
 */
export const AnUndoneRun: Story = {
  args: {
    ...RAN,
    open: true,
    undone: "Undone at 14:21:03.",
    reading: { state: "read", files: REFORMATTED, emptyNote: "This run changed nothing." },
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByRole("note")).toHaveTextContent(/Undone at 14:21:03/);
    await expect(canvas.getByText("crates/fleet/src/rehearsing/checkout.rs")).toBeVisible();
  },
};

/**
 * **A snapshot that is gone is said, and nothing stands in for it.** No patch
 * against `HEAD` is drawn — that would show a person's own uncommitted work as
 * the run's — and the header carries no count, since nothing was read.
 */
export const TheSnapshotIsGone: Story = {
  args: {
    ...RAN,
    open: true,
    reading: { state: "gone", why: "the snapshot this run kept is no longer in the repository" },
  },
  play: async ({ canvas }) => {
    await expect(canvas.getByText(/no longer in the repository/)).toBeVisible();
    await expect(canvas.getByText(/snapshot gone/)).toBeVisible();
    await expect(canvas.queryByText(/files ·/)).toBeNull();
    await expect(canvas.queryByRole("list")).toBeNull();
  },
};

/** Fleet has been asked. The header says so rather than counting nothing. */
export const Reading: Story = {
  args: { ...RAN, open: true, reading: { state: "reading" } },
};
