import type { Meta, StoryObj } from "@storybook/react-vite";
import type {
  CheckoutRunRecord,
  CheckoutRunSheet,
  RunEntry,
  ServerEntry,
} from "@armada/protocol";
import { expect, userEvent, within } from "storybook/test";
import { ManifestFrom, MANIFEST_TEXT, NOW, PULLED_TEXT } from "./Manifest";

/**
 * Bridge's Manifest surface, at `⌘4` — Journey 9's *Running one*, with no Job
 * in existence.
 *
 * **Drawn from this repository's own `armada.yml`**, in the shape
 * `GET /manifest/run_sheet` sends it: `setup.requires` names `bootstrap` and
 * `browsers`, so those two are Setup; `checks:` is drawn whole in declaration
 * order, with `format`'s own narrowing dropped because the main checkout has
 * no base to narrow against; and what is left of `commands:` — `fmt`, `gate`
 * and the `storybook_dev` server — falls to Commands.
 *
 * **Nothing on this page is hued.** No run from here writes Evidence or leaves
 * a stored pass or fail against a Check, so an exit code is a fact and never a
 * status colour — including the one below that is not the code expected.
 */
const meta = {
  title: "Screens/Manifest",
  component: ManifestFrom,
  parameters: { layout: "fullscreen" },
} satisfies Meta<typeof ManifestFrom>;

export default meta;
type Story = StoryObj<typeof meta>;

/** One row, in the shape `RunEntry` has with nothing narrowed against it. */
function entry(
  name: string,
  run: string,
  extra: Partial<RunEntry> = {},
): RunEntry {
  return {
    name,
    run,
    narrows: false,
    requires: [],
    expect_exit_code: 0,
    destructive: false,
    // Nothing in the main checkout is frozen: this is the file Fleet holds.
    frozen: false,
    ...extra,
  };
}

const STORYBOOK: ServerEntry = {
  name: "storybook_dev",
  serve: "pnpm -C packages/components exec storybook dev -p 41207 --no-open --ci",
  ready: "curl -sf http://localhost:41207",
  links: [{ url: "http://localhost:41207", name: "Storybook" }],
  destructive: false,
};

/** This repository's Manifest, as `get_checkout_run_sheet` answers it. */
function sheet(over: Partial<CheckoutRunSheet> = {}): CheckoutRunSheet {
  return {
    setup: [
      entry("bootstrap", "pnpm install --frozen-lockfile"),
      entry(
        "browsers",
        "pnpm -C packages/components exec playwright install chromium --only-shell",
      ),
    ],
    checks: [
      entry("build", "cargo build --workspace --locked", { narrows: true }),
      entry("test", "cargo nextest run --workspace --exclude acceptance", { narrows: true }),
      entry("typecheck", "pnpm typecheck"),
      entry("bridge_build", "pnpm -C apps/desktop build"),
      entry("storybook", "pnpm -C packages/components build-storybook"),
      entry("bridge_test", "pnpm bridge-test"),
      entry("format", "cargo fmt --all --check", { narrows: true }),
    ],
    commands: [
      entry("fmt", "cargo fmt --all", { destructive: true }),
      entry("gate", "cargo xtask verify-foundations", { expect_exit_code: 0 }),
    ],
    manifest_edited_at: "2026-09-10T09:14:00Z",
    servers: [STORYBOOK],
    ...over,
  };
}

/** Nothing has run: three groups, and a control that reads as an instruction. */
export const AtRest: Story = {
  name: "At rest",
  args: { sheet: { state: "read", sheet: sheet() } },
};

/**
 * A run underway. `bridge_test` is streaming, the elapsed figure is counting
 * up and Stop is the only act — the page is following `observe_checkout_run`,
 * which is opened the moment `start_checkout_run` answers.
 */
export const ARunUnderway: Story = {
  name: "A run underway",
  args: {
    sheet: {
      state: "read",
      sheet: sheet({
        running: {
          id: "crun_41c8",
          name: "bridge_test",
          command: "pnpm bridge-test",
          started_at: "2026-09-12T14:19:48Z",
        },
      }),
    },
    followed: {
      state: "following",
      runId: "crun_41c8",
      name: "bridge_test",
      path: ".armada/runs/crun_41c8/output.log",
      fromLine: 1,
      lines: [
        "> armada@0.0.0 bridge-test",
        "> pnpm -C apps/desktop test && pnpm -C packages/screens test && pnpm -C packages/components test",
        "",
        " ✓ src/main/connection.test.ts (31 tests) 812ms",
        " ✓ src/main/reader.test.ts (12 tests) 96ms",
        " ❯ src/main/rehearsal.test.ts 4/9",
      ],
    },
  },
};

/** What `fmt` rewrote, as `checkout_run.finished` reported it. */
const REFORMATTED: CheckoutRunRecord = {
  id: "crun_3ba0",
  name: "fmt",
  command: "cargo fmt --all",
  required: [],
  started_at: "2026-09-12T14:18:02Z",
  ended_at: "2026-09-12T14:18:05Z",
  duration_ms: 2740,
  exit_code: 0,
  expect_exit_code: 0,
  ended: "exited",
  stopped: false,
  changed: [
    { path: "crates/fleet/src/rehearsing/checkout.rs", change: "modified" },
    { path: "crates/api/src/rehearsing.rs", change: "modified" },
    { path: "crates/ipc/src/rehearsal.rs", change: "modified" },
  ],
  // Fleet took a snapshot before the run, so Undo has something to restore.
  undoable: true,
  log: "runs/crun_3ba0/output.log",
};

const EARLIER: CheckoutRunRecord = {
  id: "crun_2f19",
  name: "typecheck",
  command: "pnpm typecheck",
  required: [],
  started_at: "2026-09-12T14:11:30Z",
  ended_at: "2026-09-12T14:11:39Z",
  duration_ms: 9420,
  exit_code: 2,
  expect_exit_code: 0,
  ended: "exited",
  stopped: false,
  changed: [],
  undoable: false,
  log: "runs/crun_2f19/output.log",
};

/**
 * A run that changed the checkout.
 *
 * **This tree holds your own uncommitted work**, which no Job's worktree ever
 * does — so the panel lists what the run wrote and offers to put it back, and
 * Undo confirms by naming every path it would discard. There is no *Open the
 * diff*: the main checkout has no base to be read against, and nothing on the
 * wire answers for one.
 *
 * The earlier run below it exited 2 where 0 was expected, and takes no colour
 * for it. A remembered verdict here would read as Evidence the moment it sat
 * beside a Job.
 */
export const ARunThatChangedFiles: Story = {
  name: "A run that changed files",
  args: {
    sheet: { state: "read", sheet: sheet() },
    runs: { runs: [REFORMATTED, EARLIER], unreadable: [] },
    now: NOW,
  },
};

/** Fleet is up and could not read `armada.yml` — the page says so and lists nothing. */
export const TheManifestWouldNotRead: Story = {
  name: "The Manifest would not read",
  args: {
    sheet: {
      state: "failed",
      outcome: {
        ok: false,
        why: "refused",
        error: {
          code: "manifest.unreadable",
          message:
            "setup.requires names `browsers`, and commands: declares no such entry.",
          run_id: "01M1CNPKTV0018H2M1CXDNBK06",
          fields: {},
          chain: [],
        },
      },
    },
  },
};

/**
 * The file, behind the toggle named by its path — Journey 9's *Editing*.
 * **Never named by its format**: the lexicon bans calling a Manifest one.
 */
export const TheFile: Story = {
  name: "The file",
  args: { sheet: { state: "read", sheet: sheet() }, view: "file" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("tab", { name: "armada.yml" })).toHaveAttribute("aria-selected", "true");
    await expect(await canvas.findByRole("textbox", { name: /armada\.yml$/ })).toHaveValue(MANIFEST_TEXT);
    // Nothing typed, so nothing to save.
    await expect(canvas.getByRole("button", { name: "Save" })).toBeDisabled();
  },
};

/**
 * A correction Fleet would not adopt. The save lands, the watch re-reads, and
 * the refusal is drawn above the file it is about — why, then that the values
 * in force are unchanged, then every key.
 */
export const ARefusedSave: Story = {
  name: "A refused save",
  args: { sheet: { state: "read", sheet: sheet() }, view: "file", save: "refused" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const file = await canvas.findByRole("textbox", { name: /armada\.yml$/ });
    await userEvent.clear(file);
    await userEvent.type(file, MANIFEST_TEXT.replace("poke_limit: 3", "poke_limit: five"));
    await userEvent.click(canvas.getByRole("button", { name: "Save" }));
    await expect(await canvas.findByText("Manifest refused")).toBeVisible();
    await expect(canvas.getByText("drone.poke_limit")).toBeVisible();
    await expect(canvas.getByText(/The values in force are unchanged/)).toBeVisible();
  },
};

/**
 * A pull landed while the edit was open. Fleet refused the save rather than
 * take the pull with it, and both texts are drawn so neither is lost.
 */
export const ASaveOverAFileThatMoved: Story = {
  name: "A save over a file that moved",
  args: { sheet: { state: "read", sheet: sheet() }, view: "file", save: "moved" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const file = await canvas.findByRole("textbox", { name: /armada\.yml$/ });
    await userEvent.type(file, "# a note\n");
    await userEvent.click(canvas.getByRole("button", { name: "Save" }));
    await expect(await canvas.findByRole("textbox", { name: "On disk now" })).toHaveValue(PULLED_TEXT);
    await expect(canvas.getByRole("textbox", { name: "Your edit" })).toHaveValue(`${MANIFEST_TEXT}# a note\n`);
  },
};
