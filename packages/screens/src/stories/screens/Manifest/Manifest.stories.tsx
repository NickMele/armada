import type { Meta, StoryObj } from "@storybook/react-vite";
import type {
  CheckoutRunDiff,
  CheckoutRunRecord,
} from "@armada/protocol";
import { expect, fn, userEvent, within } from "storybook/test";
import {
  BUILD_FOLLOWED,
  BUILD_OUT,
  DRIFT_GONE,
  GH_ISSUE_VIEW,
  ManifestFrom,
  MANIFEST_TEXT,
  NOW,
  PULLED_TEXT,
  sheet,
  VERIFY_ENDED,
  VERIFY_UNDERWAY,
} from "./Manifest";
import { rootlessSheet, WEB_DEV_RUN } from "./rootless";

/**
 * Bridge's Manifest surface, at `⌘5` — Journey 9's *Running one*, with no Job
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

/** What `get_checkout_run_diff` answers for `fmt`: the two trees the run kept, as git wrote the patch. */
const FMT_DIFF: CheckoutRunDiff = {
  id: REFORMATTED.id,
  against: "run_snapshot",
  reading: {
    state: "read",
    files: REFORMATTED.changed,
    patch: [
      "diff --git a/crates/fleet/src/rehearsing/checkout.rs b/crates/fleet/src/rehearsing/checkout.rs",
      "index 3c1d2e0..9a7f441 100644",
      "--- a/crates/fleet/src/rehearsing/checkout.rs",
      "+++ b/crates/fleet/src/rehearsing/checkout.rs",
      "@@ -137,9 +137,7 @@ impl Fleet {",
      "         let gone = |why: &str| {",
      "-            answered(ipc::RunDiffReading::Gone {",
      "-                why: why.to_string(),",
      "-            })",
      "+            answered(ipc::RunDiffReading::Gone { why: why.to_string() })",
      "         };",
      "diff --git a/crates/api/src/rehearsing.rs b/crates/api/src/rehearsing.rs",
      "index 51e0b2a..c08d7f3 100644",
      "--- a/crates/api/src/rehearsing.rs",
      "+++ b/crates/api/src/rehearsing.rs",
      "@@ -158,7 +158,8 @@ pub(crate) async fn get_checkout_run_diff",
      "-    match served.daemon().get_checkout_run_diff(run_id).await {",
      "+    match served.daemon().get_checkout_run_diff(run_id).await",
      "+    {",
      "diff --git a/crates/ipc/src/rehearsal.rs b/crates/ipc/src/rehearsal.rs",
      "index 0d4e9b1..77a2c5e 100644",
      "--- a/crates/ipc/src/rehearsal.rs",
      "+++ b/crates/ipc/src/rehearsal.rs",
      "@@ -346,3 +346,1 @@ pub enum RunDiffReading {",
      "-    Gone {",
      "-        why: String,",
      "-    },",
      "+    Gone { why: String },",
      "",
    ].join("\n"),
  },
};

/**
 * A run that changed the checkout.
 *
 * **This tree holds your own uncommitted work**, which no Job's worktree ever
 * does — so the panel lists what the run wrote, offers **Open the diff** to
 * read how, and offers to put it back. Undo confirms by naming every path it
 * would discard.
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
    diff: FMT_DIFF,
  },
};

/**
 * **Open the diff** — the run's own patch, against the snapshot it took just
 * before it, drawn by the same `UnifiedDiff` a Job's patch is. The sheet says
 * what it is read against, because the checkout it opens over holds work that
 * is not the run's.
 */
export const TheRunsDiff: Story = {
  name: "The run's diff",
  args: { ...ARunThatChangedFiles.args },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(await canvas.findByRole("button", { name: "Open the diff" }));
    const sheet = await canvas.findByRole("dialog", { name: "What this run changed" });
    await expect(sheet).toHaveTextContent(/against the checkout just before the run/);
    await expect(within(sheet).getByText("crates/ipc/src/rehearsal.rs")).toBeVisible();
    await expect(within(sheet).getByText(/never against HEAD/)).toBeVisible();
  },
};

/**
 * **An undone run's diff is still shown, marked undone.** Undo restores from
 * the snapshot and keeps it, so the patch reads; the panel loses Undo, keeps
 * the files, and says when it was put back.
 */
export const AnUndoneRunsDiff: Story = {
  name: "An undone run's diff",
  args: {
    sheet: { state: "read", sheet: sheet() },
    runs: { runs: [{ ...REFORMATTED, undone_at: "2026-09-12T14:19:30Z" }, EARLIER], unreadable: [] },
    now: NOW,
    diff: FMT_DIFF,
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByText(/These files are back as they were/)).toBeVisible();
    await expect(canvas.queryByRole("button", { name: "Undo this run" })).toBeNull();
    await userEvent.click(canvas.getByRole("button", { name: "Open the diff" }));
    const sheet = await canvas.findByRole("dialog", { name: "What this run changed" });
    await expect(within(sheet).getByRole("note")).toHaveTextContent(/Undone at/);
    await expect(within(sheet).getByText("crates/ipc/src/rehearsal.rs")).toBeVisible();
  },
};

/**
 * **A snapshot that is gone is said plainly**, in Fleet's own words, and no
 * patch stands in for it — a diff against `HEAD` would show your own
 * uncommitted work as the run's.
 */
export const ARunWhoseSnapshotIsGone: Story = {
  name: "A run whose snapshot is gone",
  args: {
    sheet: { state: "read", sheet: sheet() },
    runs: { runs: [REFORMATTED, EARLIER], unreadable: [] },
    now: NOW,
    diff: {
      id: REFORMATTED.id,
      against: "run_snapshot",
      reading: { state: "gone", why: "the snapshot this run kept is no longer in the repository" },
    },
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(await canvas.findByRole("button", { name: "Open the diff" }));
    const sheet = await canvas.findByRole("dialog", { name: "What this run changed" });
    await expect(await within(sheet).findByText(/no longer in the repository/)).toBeVisible();
    await expect(within(sheet).queryByText("crates/ipc/src/rehearsal.rs")).toBeNull();
  },
};

/** `format` after `fmt`: a Check that changed nothing, run after a Command that did. */
const FORMAT_CHECKED: CheckoutRunRecord = {
  ...EARLIER,
  id: "crun_4d02",
  name: "format",
  command: "cargo fmt --all --check",
  started_at: "2026-09-12T14:19:10Z",
  ended_at: "2026-09-12T14:19:12Z",
  duration_ms: 1810,
  exit_code: 0,
  log: "runs/crun_4d02/output.log",
};

/**
 * **The panel is the result line's run.** `format` is newest and changed
 * nothing, so the panel says so and offers neither act — `fmt`'s files and its
 * Undo under a `format` result would read as format having written them.
 * `fmt`'s changes are still reached from *Earlier runs*.
 */
export const TheNewestRunChangedNothing: Story = {
  name: "The newest run changed nothing",
  args: {
    sheet: { state: "read", sheet: sheet() },
    runs: { runs: [FORMAT_CHECKED, REFORMATTED], unreadable: [] },
    now: NOW,
    diff: FMT_DIFF,
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByText("This run changed nothing.")).toBeVisible();
    await expect(canvas.queryByRole("button", { name: "Open the diff" })).toBeNull();
    await expect(canvas.queryByRole("button", { name: "Undo this run" })).toBeNull();
    await expect(canvas.queryByText("crates/fleet/src/rehearsing/checkout.rs")).toBeNull();
  },
};

/**
 * **Only the newest run offers Undo.** `fmt` opened from *Earlier runs*, with
 * `format` after it, shows its files and *Open the diff* and no Undo: its
 * snapshot predates the run after it, and Undo belongs to the run a person
 * just did.
 */
export const AnOlderRunFromEarlierRuns: Story = {
  name: "An older run from Earlier runs",
  args: { ...TheNewestRunChangedNothing.args },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByText("This run changed nothing.")).toBeVisible();
    const logs = canvas.getAllByRole("button", { name: /log/ });
    await userEvent.click(logs[logs.length - 1]!);
    await expect(await canvas.findByText("crates/fleet/src/rehearsing/checkout.rs")).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Open the diff" })).toBeVisible();
    await expect(canvas.queryByRole("button", { name: "Undo this run" })).toBeNull();
  },
};

/**
 * A repository-wide always-allow, beside the Commands it grew a Job's own
 * row of — #836's own case, `gh issue view` from Job 7. Removing it here
 * reaches every job against this repository at once; Fleet's own table, so
 * nothing on this page reads `armada.yml` for it.
 */
export const WithAnAlwaysAllowedCommand: Story = {
  name: "A repository-wide always-allow",
  args: { sheet: { state: "read", sheet: sheet() }, alwaysAllowed: [GH_ISSUE_VIEW] },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByText("gh issue view")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Remove gh issue view" }));
    await expect(canvas.getByText("Nothing always allowed yet.")).toBeVisible();
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
    // **The head describes the view on screen**, and moves with the toggle.
    await expect(canvas.getByText(/Save writes the file to disk and stops/)).toBeVisible();
    await expect(canvas.queryByText(/Nothing here is a verdict/)).toBeNull();
    await userEvent.click(canvas.getByRole("tab", { name: "Checks and Commands" }));
    await expect(await canvas.findByText(/Nothing here is a verdict/)).toBeVisible();
    await expect(canvas.queryByText(/Save writes the file to disk and stops/)).toBeNull();
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

/**
 * **A Check whose script the repository no longer has**, found on opening:
 * `bridge_test` runs a script `package.json` lost. `gone`, in amber, naming
 * what is missing, and nothing on the row to press.
 */
export const DriftWithAGoneLine: Story = {
  name: "Drift with a gone line",
  args: { sheet: { state: "read", sheet: sheet() }, drift: DRIFT_GONE },
  play: async ({ canvasElement }) => {
    const drift = within(within(canvasElement).getByRole("region", { name: "Drift" }));
    await expect(drift.getByText("gone")).toBeVisible();
    await expect(drift.getByText("package.json: scripts.bridge-test")).toBeVisible();
    const pressable = drift.getAllByRole("button").map((button) => button.textContent);
    await expect(pressable).toEqual(["Show the 2 lines still current"]);
  },
};

/** **A Verify underway**: `build` streams below as any run does, and no second Verify can start. */
export const AVerifyUnderway: Story = {
  name: "A Verify underway",
  args: {
    sheet: { state: "read", sheet: sheet({ running: BUILD_OUT, verify: VERIFY_UNDERWAY }) },
    followed: BUILD_FOLLOWED,
  },
  play: async ({ canvasElement }) => {
    const verify = within(within(canvasElement).getByRole("region", { name: "Verify" }));
    await expect(verify.getByRole("button", { name: "Verify" })).toBeDisabled();
    await expect(verify.getByText(/running now/)).toBeVisible();
    await expect(verify.getByRole("button", { name: "Stop" })).toBeVisible();
  },
};

/** **A Verify with a failure**: exit 2 is a fact in a chip, the Check after it still ran, and nothing is hued. */
export const AVerifyWithAFailure: Story = {
  name: "A Verify with a failure",
  args: { sheet: { state: "read", sheet: sheet({ verify: VERIFY_ENDED }) } },
  play: async ({ canvasElement }) => {
    const verify = within(within(canvasElement).getByRole("region", { name: "Verify" }));
    await expect(verify.getByText("exit 2 (expects 0)")).toBeVisible();
    await expect(verify.getByText(/Ran 4 of 4\. 1 ended with a code other than the one it expects\./)).toBeVisible();
    await expect(verify.getByRole("button", { name: "Verify" })).toBeEnabled();
  },
};

/** A repository left frozen: the page says so on every view, and the notice leads to the switch that lifts it. */
export const AFrozenRepository: Story = {
  name: "A frozen repository",
  args: { sheet: { state: "read", sheet: sheet() }, frozen: true },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByText("This repository is frozen")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "Unfreeze on the form" }));
    await expect(canvas.getByRole("tab", { name: "Edit" })).toHaveAttribute("aria-selected", "true");
    const freeze = within(await canvas.findByRole("region", { name: "Freeze" }));
    await expect(freeze.getByRole("switch", { name: /Freeze this repository/ })).toBeChecked();
    await expect(canvas.getByText("This repository is frozen")).toBeVisible();
    await expect(canvas.queryByRole("button", { name: "Unfreeze on the form" })).toBeNull();
  },
};

/**
 * A repository with no root `armada.yml`: `apps/web` declares `dev` in its own
 * file, so the page lists that alone. There is no root file to edit, drift or
 * verify, so none of those is offered, and Run names the workspace.
 */
export const AWorkspacesCommandWithNoRootManifest: Story = {
  name: "A workspace's Command, with no root Manifest",
  args: {
    sheet: { state: "read", sheet: rootlessSheet() },
    setUp: false,
    runs: { runs: [WEB_DEV_RUN], unreadable: [] },
    onStartRun: fn(() => Promise.resolve({ ok: true as const })),
  },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await expect(await canvas.findByText("In apps/web.")).toBeVisible();
    await expect(canvas.queryByRole("tab", { name: "Edit" })).toBeNull();
    await expect(canvas.queryByText("Setup")).toBeNull();
    await expect(await canvas.findByText("dev in apps/web")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: /^dev/ }));
    await userEvent.click(canvas.getByRole("button", { name: "Run" }));
    await expect(args.onStartRun).toHaveBeenCalledWith({ name: "dev", workspace: "apps/web" });
  },
};
