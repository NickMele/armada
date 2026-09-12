import type { Meta, StoryObj } from "@storybook/react-vite";
import { fn } from "storybook/test";

import { RunPage, type RunPageGroup } from "./RunPage";
import type { ConsoleRow } from "../ConsoleOutput/ConsoleOutput";
import type { ChangedFile } from "../ChangedFiles/ChangedFiles";

/**
 * Journey 9's *Running one*, on fake data built from this repository's own
 * `armada.yml`: `setup.requires` names `bootstrap` and `browsers`, so those
 * two are Setup, and everything else `commands:` declares — `fmt` and `gate` —
 * falls to Commands.
 *
 * **The same three groups as the run sheet, and none of its narrowing.** The
 * main checkout has no branch and no base, so no Check here resolves a
 * narrowed command and no row draws a scope control. What it has that the
 * sheet does not is the destructive confirmation: `fmt` writes, in the tree a
 * person is working in.
 *
 * The page fills the panel it is mounted in, so the story draws a
 * viewport-high column — the same mount `Screens/Manifest` uses.
 */
const meta: Meta<typeof RunPage> = {
  title: "Compositions/Run page",
  component: RunPage,
  decorators: [
    (Story) => (
      <div
        style={{
          display: "flex",
          flexDirection: "column",
          // `.storybook/preview.css` pads `body` by `--space-6` on every side,
          // so a frame at the full viewport height runs `2 × --space-6` past
          // the window's bottom edge before this subtracts it back out.
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

type Story = StoryObj<typeof RunPage>;

/** This repository's own Manifest, read into the page's three groups. */
const GROUPS: RunPageGroup[] = [
  {
    kind: "setup",
    label: "Setup",
    entries: [
      { id: "setup:bootstrap", name: "bootstrap", run: "pnpm install --frozen-lockfile" },
      {
        id: "setup:browsers",
        name: "browsers",
        run: "pnpm -C packages/components exec playwright install chromium --only-shell",
      },
    ],
  },
  {
    kind: "checks",
    label: "Checks",
    entries: [
      { id: "check:build", name: "build", run: "cargo build --workspace --locked" },
      { id: "check:test", name: "test", run: "cargo nextest run --workspace --exclude acceptance" },
      { id: "check:typecheck", name: "typecheck", run: "pnpm typecheck" },
      { id: "check:bridge_build", name: "bridge_build", run: "pnpm -C apps/desktop build" },
      {
        id: "check:storybook",
        name: "storybook",
        run: "pnpm -C packages/components build-storybook",
      },
      { id: "check:bridge_test", name: "bridge_test", run: "pnpm bridge-test" },
      {
        id: "check:format",
        name: "format",
        run: "cargo fmt --all --check",
        note: "Runs fmt first.",
      },
    ],
  },
  {
    kind: "commands",
    label: "Commands",
    entries: [
      // The one row on this page the flag means something on: it rewrites
      // every Rust file in the tree a person is working in.
      { id: "command:fmt", name: "fmt", run: "cargo fmt --all", destructive: true },
      { id: "command:gate", name: "gate", run: "cargo xtask verify-foundations" },
      {
        id: "server:storybook",
        name: "storybook",
        run: "pnpm -C packages/components exec storybook dev -p 41207 --no-open --ci",
      },
    ],
  },
];

const EDITED = { manifestEditedAt: "armada.yml last edited 2d ago" };

/** Nothing picked — the control reads as an instruction, not a dead form. */
export const AtRest: Story = {
  args: { ...EDITED, groups: GROUPS, onSelect: fn(), onRun: fn() },
};

// Cut mid-build rather than at a result: a run in flight has not finished, and
// a streamed reading already showing `test result: ok` would say otherwise.
const RUNNING_ROWS: ConsoleRow[] = [
  { row: "line", at: 1, text: "   Compiling armada-fleet v0.1.0" },
  { row: "fold", at: 2, says: "compiling chrono-lite v0.4.2 (18.4s) — 62 lines", open: false },
  { row: "line", at: 64, text: "   Compiling armada-api v0.1.0" },
];

/** A run underway: output streaming, elapsed counting up, and Stop. */
export const Running: Story = {
  args: {
    ...EDITED,
    groups: GROUPS,
    selectedId: "check:test",
    onSelect: fn(),
    onRun: fn(),
    onDismiss: fn(),
    running: { elapsed: "6.1s elapsed", onStop: fn() },
    output: { rows: RUNNING_ROWS, following: true },
  },
};

/** Finished, with an exit code other than the one expected — unhued either way. */
export const UnexpectedExit: Story = {
  args: {
    ...EDITED,
    groups: GROUPS,
    selectedId: "check:test",
    onSelect: fn(),
    onRun: fn(),
    onDismiss: fn(),
    output: {
      rows: [
        { row: "line", at: 1, text: "test result: FAILED. 311 passed; 1 failed; 3 ignored" },
        {
          row: "line",
          at: 2,
          text: "thread 'checking::retrying' panicked at crates/fleet/src/tests/retrying.rs:41",
        },
      ],
    },
    result: { name: "test", exitCode: 101, expected: 0, ended: "exited", duration: "41s" },
    runs: [
      {
        id: "run_7f",
        name: "typecheck",
        result: "exit 0 (expects 0)",
        time: "14:02:11",
        duration: "9.4s",
        onOpen: fn(),
      },
    ],
  },
};

// What `fmt` actually touches: Rust source, run through rustfmt.
const REFORMATTED: ChangedFile[] = [
  { path: "crates/fleet/src/rehearsing/checkout.rs", change: "modified" },
  { path: "crates/api/src/rehearsing.rs", change: "modified" },
  { path: "crates/ipc/src/rehearsal.rs", change: "modified" },
];

/**
 * A run that changed the checkout. **Undo is offered and nothing else is** —
 * there is no *Open the diff* here, because the main checkout has no base to
 * be read against and nothing on the wire answers for one. Pressing Undo puts
 * up the confirmation that names every path it would discard.
 */
export const AfterARunThatChangedFiles: Story = {
  args: {
    ...EDITED,
    groups: GROUPS,
    selectedId: "command:fmt",
    onSelect: fn(),
    onRun: fn(),
    onDismiss: fn(),
    output: { rows: [{ row: "line", at: 1, text: "Diff in crates/fleet/src/rehearsing/checkout.rs" }] },
    result: { name: "fmt", exitCode: 0, expected: 0, ended: "exited", duration: "2.7s" },
    changed: { files: REFORMATTED, onUndo: fn() },
  },
};

/** A Command with `serve:`, up and answering, with its links and Stop. */
export const AServerServing: Story = {
  args: {
    ...EDITED,
    groups: GROUPS,
    selectedId: "server:storybook",
    onSelect: fn(),
    onRun: fn(),
    onStopServer: fn(),
    onOpenLink: fn(),
    server: {
      phase: "serving",
      address: "localhost:41207",
      uptime: "up 4m",
      links: [{ url: "http://localhost:41207", name: "Storybook" }],
    },
  },
};
