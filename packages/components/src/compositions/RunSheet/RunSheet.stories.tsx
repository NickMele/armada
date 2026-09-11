import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, fn } from "storybook/test";

import { RunSheet, type RunSheetGroup } from "./RunSheet";
import type { ConsoleRow } from "../ConsoleOutput/ConsoleOutput";
import type { ChangedFile } from "../ChangedFiles/ChangedFiles";

/**
 * Journey 9's *Running one inside a Job*, on fake data built from this
 * repository's own `armada.yml`: `setup.requires` names `bootstrap` and
 * `browsers`, so those two are Setup and everything else `commands:` declares
 * — `fmt` and `gate` — falls to Commands. `checks:` is drawn whole, in
 * declaration order, with `format`'s own `requires: [fmt]` and `typecheck`'s
 * `when` both carried into the row note.
 *
 * The sheet is laid out inside the nearest positioned ancestor, so the story
 * draws one — the same convention `JobDiffSheet`'s stories use.
 */
const meta: Meta<typeof RunSheet> = {
  title: "Compositions/Run sheet",
  component: RunSheet,
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

type Story = StoryObj<typeof RunSheet>;

const WHEN_BRIDGE = [
  "apps/**",
  "packages/**",
  "crates/core-model/domain/**",
  "protocol-version.toml",
  "package.json",
  "pnpm-lock.yaml",
  "pnpm-workspace.yaml",
];

/** This repository's own Manifest, read into the sheet's three groups. */
const GROUPS: RunSheetGroup[] = [
  {
    kind: "setup",
    label: "Setup",
    entries: [
      { id: "bootstrap", name: "bootstrap", run: "pnpm install --frozen-lockfile" },
      {
        id: "browsers",
        name: "browsers",
        run: "pnpm -C packages/components exec playwright install chromium --only-shell",
      },
    ],
  },
  {
    kind: "checks",
    label: "Checks",
    entries: [
      {
        id: "build",
        name: "build",
        run: "cargo build --workspace --locked",
        narrowRun: "cargo build --locked",
        narrowed: true,
      },
      {
        id: "test",
        name: "test",
        run: "cargo nextest run --workspace --exclude acceptance",
        narrowRun: "cargo nextest run",
        narrowed: true,
      },
      {
        id: "typecheck",
        name: "typecheck",
        run: "pnpm typecheck",
        note: `Skipped for this Job — no changed file is under ${WHEN_BRIDGE.join(", ")}.`,
      },
      { id: "bridge_build", name: "bridge_build", run: "pnpm -C apps/desktop build" },
      { id: "storybook", name: "storybook", run: "pnpm -C packages/components build-storybook" },
      { id: "bridge_test", name: "bridge_test", run: "pnpm bridge-test" },
      {
        id: "format",
        name: "format",
        run: "cargo fmt --all --check",
        note: "Runs fmt first.",
        narrowRun: "rustfmt --check --edition 2021",
        narrowed: true,
      },
    ],
  },
  {
    kind: "commands",
    label: "Commands",
    entries: [
      { id: "fmt", name: "fmt", run: "cargo fmt --all" },
      { id: "gate", name: "gate", run: "cargo xtask verify-foundations" },
    ],
  },
];

const HEADER = { jobName: "job_2d90bb", manifestEditedAt: "armada.yml last edited 2d ago" };

/** Nothing selected — the control reads as an instruction rather than a dead form. */
export const AtRest: Story = {
  args: {
    open: true,
    ...HEADER,
    groups: GROUPS,
    onSelect: fn(),
  },
};

const RUNNING_ROWS: ConsoleRow[] = [
  { row: "line", at: 1, text: "   Compiling armada-fleet v0.1.0" },
  { row: "fold", at: 2, says: "compiling chrono-lite v0.4.2 (18.4s) — 62 lines", open: false },
  { row: "line", at: 64, text: "test result: ok. 312 passed; 0 failed; 3 ignored" },
];

/** A run in flight: output streaming, elapsed counting up, and Stop. */
export const Running: Story = {
  args: {
    open: true,
    ...HEADER,
    groups: GROUPS,
    selectedId: "test",
    onSelect: fn(),
    onToggleNarrow: fn(),
    onRun: fn(),
    running: { elapsed: "6.1s elapsed", onStop: fn() },
    output: { rows: RUNNING_ROWS, following: true },
  },
};

/** Finished, with an exit code other than the one expected — unhued either way. */
export const UnexpectedExit: Story = {
  args: {
    open: true,
    ...HEADER,
    groups: GROUPS,
    selectedId: "test",
    onSelect: fn(),
    onToggleNarrow: fn(),
    onRun: fn(),
    output: {
      rows: [
        { row: "line", at: 1, text: "test result: FAILED. 311 passed; 1 failed; 3 ignored" },
        { row: "line", at: 2, text: "thread 'checking::retrying' panicked at crates/fleet/src/tests/retrying.rs:41" },
      ],
    },
    result: { name: "test", exitCode: 101, expected: 0, duration: "41s" },
  },
};

const CHANGED_FILES: ChangedFile[] = [
  { path: "Cargo.lock", change: "modified", added: 4, deleted: 1 },
  { path: "packages/components/package.json", change: "modified", added: 1, deleted: 0 },
];

/** A run that wrote to the tree: the files, Open the diff, Undo this run. */
export const ChangedTheTree: Story = {
  args: {
    open: true,
    ...HEADER,
    groups: GROUPS,
    selectedId: "fmt",
    onSelect: fn(),
    onRun: fn(),
    output: { rows: [], emptyNote: "cargo fmt wrote no output." },
    result: { name: "fmt", exitCode: 0, expected: 0, duration: "3.2s" },
    changed: {
      files: CHANGED_FILES,
      onOpenDiff: fn(),
      onUndo: fn(),
    },
  },
};

/**
 * The same run, with a Drone working in this tree. **No Undo, and the Drone
 * notice** — Undo needs a snapshot taken before the run, and a Drone's work is
 * uncommitted until delivery, so a plain discard would take it too.
 */
export const ChangedTheTreeWithADroneWorking: Story = {
  args: {
    ...ChangedTheTree.args,
    droneWorking: true,
  },
};

/** The worktree's own `armada.yml` differs from the one this Job froze. */
export const ManifestDiffers: Story = {
  args: {
    open: true,
    ...HEADER,
    groups: GROUPS,
    onSelect: fn(),
    manifestDiffers: { onUseWorktreeVersion: fn() },
  },
};

/** A Check the gate skips for this Job — the row's own sentence, never a bare tag. */
export const ASkippedCheck: Story = {
  args: {
    open: true,
    ...HEADER,
    groups: GROUPS,
    selectedId: "typecheck",
    onSelect: fn(),
  },
};

/**
 * Pressing **Run** reports the selected entry and whether it was narrowed.
 *
 * **What earns this a play**: nothing on the rendering says which entry a
 * press acts on, or whether the resolved command is the narrowed one — both
 * are behaviour, not a colour or a measurement.
 */
export const PressingRunReportsTheSelection: Story = {
  args: {
    open: true,
    ...HEADER,
    groups: GROUPS,
    selectedId: "build",
    onSelect: fn(),
    onToggleNarrow: fn(),
    onRun: fn(),
  },
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Run" }));
    await expect(args.onRun).toHaveBeenCalledWith({ id: "build", narrowed: true });
  },
};

/**
 * **Run the whole tree instead** changes what **Run** reports.
 *
 * The sheet holds no state of its own — narrowing is the caller's — so the
 * story is the caller: `onToggleNarrow` flips a held flag, the same way
 * `HeldWorktree`'s selection story hands a controlled prop back to itself.
 * What earns this a play is exactly what a rendering cannot show: pressing one
 * control changes what a *later* press of a different control reports.
 */
export const RunTheWholeTreeInstead: Story = {
  render: () => {
    const [narrowed, setNarrowed] = useState(true);
    const [reported, setReported] = useState<{ id: string; narrowed: boolean } | null>(null);
    const groups = GROUPS.map((group) =>
      group.kind === "checks"
        ? {
            ...group,
            entries: group.entries.map((entry) =>
              entry.id === "build" ? { ...entry, narrowed } : entry,
            ),
          }
        : group,
    );
    return (
      <>
        <RunSheet
          open
          {...HEADER}
          groups={groups}
          selectedId="build"
          onSelect={() => {}}
          onToggleNarrow={() => setNarrowed((was) => !was)}
          onRun={setReported}
        />
        <p data-testid="reported" style={{ position: "absolute", inset: "auto 0 0 0" }}>
          {reported === null ? "nothing run yet" : JSON.stringify(reported)}
        </p>
      </>
    );
  },
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Run the whole tree instead" }));
    await userEvent.click(canvas.getByRole("button", { name: "Run" }));
    await expect(canvas.getByTestId("reported")).toHaveTextContent(
      JSON.stringify({ id: "build", narrowed: false }),
    );
  },
};

/**
 * **Undo this run is absent while a Drone is working**, whatever the caller
 * passed for `onUndo` — the rule the journey states rather than a choice this
 * story makes.
 */
export const UndoIsAbsentWhileADroneWorks: Story = {
  args: { ...ChangedTheTree.args, droneWorking: true },
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("button", { name: "Undo this run" })).not.toBeInTheDocument();
    await expect(canvas.getByRole("button", { name: "Open the diff" })).toBeVisible();
  },
};
