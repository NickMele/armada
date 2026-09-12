import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, fn } from "storybook/test";

import { RunSheet, type RunSheetGroup, type RunSheetProps } from "./RunSheet";
import type { ConsoleRow } from "../ConsoleOutput/ConsoleOutput";
import type { ChangedFile } from "../ChangedFiles/ChangedFiles";

/**
 * Journey 9's *Running one inside a Job*, on fake data built from this
 * repository's own `armada.yml`: `setup.requires` names `bootstrap` and
 * `browsers`, so those two are Setup and everything else `commands:` declares
 * — `fmt` and `gate` — falls to Commands. `checks:` is drawn whole, in
 * declaration order, with `format`'s own `requires: [fmt]` carried into the
 * row note. **No note for what `when` skips** — Nick's own reading: this
 * panel is for running things by hand, and a `when` clause's whole path list
 * is prose for auditing the gate, not for a row read at a glance.
 *
 * The sheet is laid out inside the nearest positioned ancestor, so the story
 * draws one — the same convention `JobDiffSheet`'s stories use. Not a token
 * for the height: this rail is three full groups deep and
 * `--palette-max-height` (400px, the command palette's own ceiling) clipped
 * it before `storybook` ever came into view. No length token names a
 * full-height reading, and a viewport unit is not the length literal the
 * gate polices — it is the viewport, which is what a sheet actually opens
 * against.
 */
const meta: Meta<typeof RunSheet> = {
  title: "Compositions/Run sheet",
  component: RunSheet,
  decorators: [
    (Story) => (
      <div
        style={{
          position: "relative",
          // `.storybook/preview.css` pads `body` by `--space-6` on every
          // side, so a frame at the full viewport height runs `2 ×
          // --space-6` past the window's bottom edge before this subtracts
          // it back out.
          height: "calc(100dvh - 2 * var(--space-6))",
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
        narrowRun: "cargo build --locked -p fleet -p api",
        narrowed: true,
      },
      {
        id: "test",
        name: "test",
        run: "cargo nextest run --workspace --exclude acceptance",
        narrowRun: "cargo nextest run -p fleet -p api",
        narrowed: true,
      },
      { id: "typecheck", name: "typecheck", run: "pnpm typecheck" },
      { id: "bridge_build", name: "bridge_build", run: "pnpm -C apps/desktop build" },
      { id: "storybook", name: "storybook", run: "pnpm -C packages/components build-storybook" },
      { id: "bridge_test", name: "bridge_test", run: "pnpm bridge-test" },
      {
        id: "format",
        name: "format",
        run: "cargo fmt --all --check",
        note: "Runs fmt first.",
        narrowRun:
          "rustfmt --check --edition 2021 crates/fleet/src/working/dry_run.rs " +
          "crates/api/src/routes/jobs.rs",
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
      // A Command with `serve:` is a server. The rail row shows the `serve`
      // line itself, resolved — `${port.storybook}` is a claim from the Job's
      // own span, and 41207 is what that claim came to on this run.
      {
        id: "storybook_server",
        name: "storybook",
        run: "pnpm -C packages/components exec storybook dev -p 41207 --no-open --ci",
      },
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

// Cut mid-build, not at a result — a running Job has not finished, and a
// streamed reading that already showed `test result: ok` would say otherwise.
const RUNNING_ROWS: ConsoleRow[] = [
  { row: "line", at: 1, text: "   Compiling armada-fleet v0.1.0" },
  { row: "fold", at: 2, says: "compiling chrono-lite v0.4.2 (18.4s) — 62 lines", open: false },
  { row: "line", at: 64, text: "   Compiling armada-api v0.1.0" },
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

// A thousand lines, so the panel's own ceiling is the thing being read
// rather than assumed. `test result: ok` never arrives — see `Running`'s own
// note on why a streamed reading stops short of the line that claims it is
// over.
const THOUSAND_LINES: ConsoleRow[] = Array.from({ length: 1000 }, (_, i) => ({
  row: "line",
  at: i + 1,
  text: `test perimeter::sweep_${String(i + 1).padStart(4, "0")} ... ok`,
}));

/**
 * The output panel fills the remaining height of the main column and scrolls
 * inside itself — the sheet never grows for it, whatever the reading holds.
 */
export const OutputWithAThousandLines: Story = {
  args: {
    open: true,
    ...HEADER,
    groups: GROUPS,
    selectedId: "test",
    onSelect: fn(),
    onToggleNarrow: fn(),
    onRun: fn(),
    running: { elapsed: "38.4s elapsed", onStop: fn() },
    output: { rows: THOUSAND_LINES, following: true },
  },
};

// A line long enough that no column width would hold it — the case the sheet's
// own "reading" size and the console's own non-wrapping text both exist for.
const LONG_LINE_ROWS: ConsoleRow[] = [
  { row: "line", at: 1, text: "   Compiling armada-fleet v0.1.0" },
  {
    row: "line",
    at: 2,
    text:
      "thread 'checking::retrying::dispatch_when_the_worktree_already_carries_an_untracked_file_" +
      "the_prior_attempt_left_behind' panicked at crates/fleet/src/tests/retrying.rs:412:9: " +
      "assertion `left == right` failed: expected the retry to inherit the prior attempt's " +
      "worktree unchanged, found 3 files modified outside the diff the Job itself produced",
  },
  { row: "line", at: 3, text: "test result: FAILED. 311 passed; 1 failed; 3 ignored" },
];

/**
 * A line long enough to prove the point rather than assume it: the row
 * scrolls sideways under `.armada-console__rows`' own `overflow-x: auto`
 * instead of wrapping, and the line number stays pinned at the near edge
 * while it does — `position: sticky` on `.armada-console__at`, with a
 * background matched to the row so the scrolling text disappears behind the
 * number rather than showing through it.
 */
export const OutputWithALongLine: Story = {
  args: {
    open: true,
    ...HEADER,
    groups: GROUPS,
    selectedId: "test",
    onSelect: fn(),
    onToggleNarrow: fn(),
    onRun: fn(),
    output: { rows: LONG_LINE_ROWS },
    result: { name: "test", exitCode: 101, expected: 0, duration: "41s" },
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

// What `fmt` actually touches: Rust source, run through rustfmt. Neither
// Cargo.lock nor a package.json is a file `cargo fmt` ever rewrites.
const CHANGED_FILES: ChangedFile[] = [
  { path: "crates/fleet/src/working/dry_run.rs", change: "modified", added: 3, deleted: 3 },
  { path: "crates/api/src/routes/jobs.rs", change: "modified", added: 2, deleted: 1 },
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
 * Choosing **All** on the segmented control changes what **Run** reports.
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
    await userEvent.click(canvas.getByRole("radio", { name: "All" }));
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

// --- A server: a Command declaring `serve` --------------------------------
//
// Journey 9's "A server" section. `storybook_server` above is the fixture:
// the Manifest's own storybook example, on a port leased from this Job's
// span. The header is the same job every other story in this file uses.

const STARTING_OUTPUT: ConsoleRow[] = [
  { row: "line", at: 1, text: "$ pnpm -C packages/components exec storybook dev -p 41207 --no-open --ci" },
  { row: "line", at: 2, text: "@storybook/core v10.5.10" },
];

/**
 * Before `ready` passes: the row reads *starting*, the log streams, and
 * **no link buttons yet** — pressing one before something answers on the port
 * would open an address nothing is serving.
 */
export const ServerStarting: Story = {
  args: {
    open: true,
    ...HEADER,
    groups: GROUPS,
    selectedId: "storybook_server",
    onSelect: fn(),
    onRun: fn(),
    server: { phase: "starting" },
    output: { rows: STARTING_OUTPUT, following: true },
  },
};

const SERVING_OUTPUT: ConsoleRow[] = [
  { row: "line", at: 1, text: "╭──────────────────────────────────────────────╮" },
  { row: "line", at: 2, text: "│  Storybook 10.5.10 started                    │" },
  { row: "line", at: 3, text: "│  Local:  http://localhost:41207/              │" },
  { row: "line", at: 4, text: "╰──────────────────────────────────────────────╯" },
];

/** Serving, with one link: *serving*, how long it has been up, and Stop. */
export const ServerServing: Story = {
  args: {
    open: true,
    ...HEADER,
    groups: GROUPS,
    selectedId: "storybook_server",
    onSelect: fn(),
    onRun: fn(),
    server: {
      phase: "serving",
      address: "localhost:41207",
      uptime: "up 4m",
      links: [{ url: "http://localhost:41207", name: "Storybook" }],
    },
    onOpenLink: fn(),
    onStopServer: fn(),
    output: { rows: SERVING_OUTPUT },
  },
};

/**
 * Serving, with two named links — a link with no `name` in the Manifest draws
 * the bare URL instead, so the button is never unlabelled.
 */
export const ServerServingTwoLinks: Story = {
  args: {
    ...ServerServing.args,
    server: {
      phase: "serving",
      address: "localhost:41207",
      uptime: "up 12m",
      links: [
        { url: "http://localhost:41207", name: "Storybook" },
        { url: "http://localhost:41207/coverage" },
      ],
    },
  },
};

/** The same server, started by a Drone through Fleet's MCP tool — the same row, with Stop, and a short note. */
export const ServerStartedByADrone: Story = {
  args: {
    ...ServerServing.args,
    server: {
      phase: "serving",
      address: "localhost:41207",
      uptime: "up 4m",
      links: [{ url: "http://localhost:41207", name: "Storybook" }],
      startedByDrone: true,
    },
  },
};

/**
 * **A server that exits on its own has failed, whatever its exit code** —
 * worded as stopped on its own, and unhued: a server's activity carries no
 * status token, so words carry the reading the way an unhued exit code does
 * everywhere else on this sheet.
 */
export const ServerExitedOnItsOwn: Story = {
  args: {
    open: true,
    ...HEADER,
    groups: GROUPS,
    selectedId: "storybook_server",
    onSelect: fn(),
    onRun: fn(),
    server: { phase: "exited", exitCode: 1 },
    output: {
      rows: [
        { row: "line", at: 1, text: "Error: listen EADDRINUSE: address already in use :::41207" },
      ],
    },
  },
};

/** Pressing a link reports its URL, and never navigates. */
export const PressingALinkReportsItsUrl: Story = {
  args: ServerServing.args,
  play: async ({ args, canvas, userEvent }) => {
    const link = canvas.getByRole("button", { name: "Storybook" });
    // A link is a button, never an anchor — nothing here carries an `href`
    // for a click to follow.
    await expect(link.tagName).toBe("BUTTON");
    await userEvent.click(link);
    await expect(args.onOpenLink).toHaveBeenCalledWith("http://localhost:41207");
  },
};

/** Stop on a serving server reports which one. */
export const StopReportsTheServer: Story = {
  args: ServerServing.args,
  play: async ({ args, canvas, userEvent }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Stop" }));
    await expect(args.onStopServer).toHaveBeenCalledWith("storybook_server");
  },
};

/**
 * Selecting the server row, and starting it, must not carry `fmt`'s own run
 * under the serving bar — Nick's own report, from a live pass against a real
 * Fleet: the bar read `serving` and `Stop`, correct and live, with `fmt`'s log
 * and *Printed nothing.* sitting beneath it as if that were the server's own
 * reading.
 *
 * **A small caller, not a fixture.** The sheet itself holds no state — see
 * `RunTheWholeTreeInstead` above — so this story is one: `output` reads a
 * followed run's lines only where its name is the entry now selected, the
 * same rule `rehearsal.ts`'s `runOutputOf` applies for the real sheet. A hook
 * has no Storybook page of its own, so this is where the rule is proven
 * against a render rather than only against its own return value.
 */
export const ServerOutputDoesNotInheritAPreviousRun: Story = {
  render: () => {
    const [selectedId, setSelectedId] = useState("fmt");
    // `fmt` printed a line and is still the thing a socket is following —
    // nothing about picking a different row stops Fleet's own stream.
    const followed = { name: "fmt", rows: [{ row: "line" as const, at: 1, text: "Fixed 3 files" }] };
    const [server, setServer] = useState<RunSheetProps["server"]>(undefined);

    return (
      <RunSheet
        open
        {...HEADER}
        groups={GROUPS}
        selectedId={selectedId}
        onSelect={setSelectedId}
        onRun={({ id }) => {
          if (id === "storybook_server") {
            setServer({ phase: "serving", address: "localhost:41207", uptime: "up 4s", links: [] });
          }
        }}
        // The gate itself: a followed run's lines are read only where its
        // name is the entry now selected.
        output={followed.name === selectedId ? { rows: followed.rows } : undefined}
        server={selectedId === "storybook_server" ? server : undefined}
      />
    );
  },
  play: async ({ canvas, userEvent }) => {
    await expect(canvas.getByText("Fixed 3 files")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: /storybook dev/ }));
    await expect(canvas.queryByText("Fixed 3 files")).not.toBeInTheDocument();
    await userEvent.click(canvas.getByRole("button", { name: "Run" }));
    await expect(canvas.getByText("localhost:41207")).toBeVisible();
    await expect(canvas.queryByText("Fixed 3 files")).not.toBeInTheDocument();
  },
};

/** No link buttons while starting — there is nothing yet to press one against. */
export const NoLinksWhileStarting: Story = {
  args: ServerStarting.args,
  play: async ({ canvas }) => {
    await expect(canvas.queryByRole("button", { name: "Storybook" })).not.toBeInTheDocument();
    await expect(canvas.queryByRole("button", { name: "Stop" })).not.toBeInTheDocument();
    await expect(canvas.getByText("Starting.")).toBeVisible();
  },
};
