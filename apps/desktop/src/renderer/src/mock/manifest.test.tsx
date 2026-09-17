// The Manifest surface, through `App` — Journey 9's *Running one* and
// *Editing*, with no Job in existence. Moved here from `Screens/Manifest`'s
// stories, which drew the surface from a copy of `App`'s wiring — #1224.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { CheckoutRunDiff, CheckoutRunRecord } from "@armada/protocol";

import {
  BUILD_FOLLOWED,
  BUILD_OUT,
  DRIFT_GONE,
  GH_ISSUE_VIEW,
  MANIFEST_TEXT,
  PULLED_TEXT,
  VERIFY_ENDED,
  VERIFY_UNDERWAY,
  manifesting,
  sheet,
} from "./manifest-fleet";
import type { Manifesting } from "./manifest-fleet";
import { rootlessSheet, WEB_DEV_RUN } from "./manifest-rootless";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

/** The Manifest surface, by the rail. */
async function manifest(options: Manifesting = {}): Promise<void> {
  mount(manifesting(options));
  await page.getByRole("button", { name: "Manifest", exact: true }).click();
}

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

/** What `get_checkout_run_diff` answers for `fmt`. */
const FMT_DIFF: CheckoutRunDiff = {
  id: REFORMATTED.id,
  against: "run_snapshot",
  reading: {
    state: "read",
    files: REFORMATTED.changed,
    patch: [
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

const diffSheet = () => page.getByRole("dialog", { name: "What this run changed" });

test("Open the diff reads the run's own patch, against the checkout just before it", async () => {
  await manifest({ runs: { runs: [REFORMATTED, EARLIER], unreadable: [] }, diff: FMT_DIFF });
  await page.getByRole("button", { name: "Open the diff" }).click();
  await expect.element(diffSheet()).toHaveTextContent(/against the checkout just before the run/);
  await expect.element(diffSheet().getByText("crates/ipc/src/rehearsal.rs").first()).toBeVisible();
  await expect.element(diffSheet().getByText(/never against HEAD/)).toBeVisible();
});

test("an undone run's diff is still shown, marked undone, and Undo is gone", async () => {
  await manifest({
    runs: { runs: [{ ...REFORMATTED, undone_at: "2026-09-12T14:19:30Z" }, EARLIER], unreadable: [] },
    diff: FMT_DIFF,
  });
  await expect.element(page.getByText(/These files are back as they were/)).toBeVisible();
  expect(page.getByRole("button", { name: "Undo this run" }).query()).toBeNull();
  await page.getByRole("button", { name: "Open the diff" }).click();
  await expect.element(diffSheet().getByRole("note")).toHaveTextContent(/Undone at/);
  await expect.element(diffSheet().getByText("crates/ipc/src/rehearsal.rs").first()).toBeVisible();
});

test("a snapshot that is gone is said plainly, and no patch stands in for it", async () => {
  await manifest({
    runs: { runs: [REFORMATTED, EARLIER], unreadable: [] },
    diff: {
      id: REFORMATTED.id,
      against: "run_snapshot",
      reading: { state: "gone", why: "the snapshot this run kept is no longer in the repository" },
    },
  });
  await page.getByRole("button", { name: "Open the diff" }).click();
  await expect.element(diffSheet().getByText(/no longer in the repository/)).toBeVisible();
  expect(diffSheet().getByText("crates/ipc/src/rehearsal.rs").query()).toBeNull();
});

test("the newest run changed nothing, so the panel offers neither the diff nor Undo", async () => {
  await manifest({ runs: { runs: [FORMAT_CHECKED, REFORMATTED], unreadable: [] }, diff: FMT_DIFF });
  await expect.element(page.getByText("This run changed nothing.")).toBeVisible();
  expect(page.getByRole("button", { name: "Open the diff" }).query()).toBeNull();
  expect(page.getByRole("button", { name: "Undo this run" }).query()).toBeNull();
  expect(page.getByText("crates/fleet/src/rehearsing/checkout.rs").query()).toBeNull();
});

test("an older run opened from Earlier runs shows its files and diff, and no Undo", async () => {
  await manifest({ runs: { runs: [FORMAT_CHECKED, REFORMATTED], unreadable: [] }, diff: FMT_DIFF });
  await expect.element(page.getByText("This run changed nothing.")).toBeVisible();
  const logs = page.getByRole("button", { name: /log/ }).elements();
  await userEvent.click(logs[logs.length - 1]!);
  await expect.element(page.getByText("crates/fleet/src/rehearsing/checkout.rs")).toBeVisible();
  await expect.element(page.getByRole("button", { name: "Open the diff" })).toBeVisible();
  expect(page.getByRole("button", { name: "Undo this run" }).query()).toBeNull();
});

test("a repository-wide always-allow is listed, and removed from here", async () => {
  await manifest({ alwaysAllowed: [GH_ISSUE_VIEW] });
  await expect.element(page.getByText("gh issue view")).toBeVisible();
  await page.getByRole("button", { name: "Remove gh issue view" }).click();
  await expect.element(page.getByText("Nothing always allowed yet.")).toBeVisible();
});

test("the file sits behind a tab named by its path, and the head moves with the tab", async () => {
  await manifest();
  await page.getByRole("tab", { name: "armada.yml" }).click();
  await expect.element(page.getByRole("tab", { name: "armada.yml" })).toHaveAttribute("aria-selected", "true");
  await expect.element(page.getByRole("textbox", { name: /armada\.yml$/ })).toHaveValue(MANIFEST_TEXT);
  await expect.element(page.getByRole("button", { name: "Save" })).toBeDisabled();
  await expect.element(page.getByText(/Save writes the file to disk and stops/)).toBeVisible();
  expect(page.getByText(/Nothing here is a verdict/).query()).toBeNull();
  await page.getByRole("tab", { name: "Checks and Commands" }).click();
  await expect.element(page.getByText(/Nothing here is a verdict/)).toBeVisible();
  await expect.poll(() => page.getByText(/Save writes the file to disk and stops/).query()).toBeNull();
});

test("a save Fleet would not adopt draws the refusal, the values in force, and every key", async () => {
  await manifest({ save: "refused" });
  await page.getByRole("tab", { name: "armada.yml" }).click();
  const file = page.getByRole("textbox", { name: /armada\.yml$/ });
  await expect.element(file).toHaveValue(MANIFEST_TEXT);
  await file.clear();
  await userEvent.type(file, MANIFEST_TEXT.replace("poke_limit: 3", "poke_limit: five"));
  await page.getByRole("button", { name: "Save" }).click();
  await expect.element(page.getByText("Manifest refused")).toBeVisible();
  await expect.element(page.getByText("drone.poke_limit")).toBeVisible();
  await expect.element(page.getByText(/The values in force are unchanged/)).toBeVisible();
});

test("a save over a file that moved draws both texts, so neither is lost", async () => {
  await manifest({ save: "moved" });
  await page.getByRole("tab", { name: "armada.yml" }).click();
  const file = page.getByRole("textbox", { name: /armada\.yml$/ });
  await expect.element(file).toHaveValue(MANIFEST_TEXT);
  // Typed at the end of the file, where a person appending a note puts the cursor.
  const text = file.element() as HTMLTextAreaElement;
  text.focus();
  text.setSelectionRange(text.value.length, text.value.length);
  await userEvent.keyboard("# a note{Enter}");
  await page.getByRole("button", { name: "Save" }).click();
  await expect.element(page.getByRole("textbox", { name: "On disk now" })).toHaveValue(PULLED_TEXT);
  await expect.element(page.getByRole("textbox", { name: "Your edit" })).toHaveValue(`${MANIFEST_TEXT}# a note\n`);
});

test("drift with a gone line names what is missing, and offers nothing to press on the row", async () => {
  await manifest({ drift: DRIFT_GONE });
  const drift = page.getByRole("region", { name: "Drift" });
  await expect.element(drift.getByText("gone", { exact: true })).toBeVisible();
  await expect.element(drift.getByText("package.json: scripts.bridge-test")).toBeVisible();
  expect(drift.getByRole("button").elements().map((button) => button.textContent)).toEqual(["Show the 2 lines still current"]);
});

test("a Verify underway: build streams, and no second Verify can start", async () => {
  await manifest({ sheet: { state: "read", sheet: sheet({ running: BUILD_OUT, verify: VERIFY_UNDERWAY }) }, followed: BUILD_FOLLOWED });
  const verify = page.getByRole("region", { name: "Verify" });
  await expect.element(verify.getByRole("button", { name: "Verify" })).toBeDisabled();
  await expect.element(verify.getByText(/running now/)).toBeVisible();
  await expect.element(verify.getByRole("button", { name: "Stop" })).toBeVisible();
});

test("a Verify with a failure: the exit code is a fact, and the Check after it still ran", async () => {
  await manifest({ sheet: { state: "read", sheet: sheet({ verify: VERIFY_ENDED }) } });
  const verify = page.getByRole("region", { name: "Verify" });
  await expect.element(verify.getByText("exit 2 (expects 0)")).toBeVisible();
  await expect.element(verify.getByText(/Ran 4 of 4\. 1 ended with a code other than the one it expects\./)).toBeVisible();
  await expect.element(verify.getByRole("button", { name: "Verify" })).toBeEnabled();
});

test("a frozen repository says so, and the notice leads to the switch that lifts it", async () => {
  await manifest({ frozen: true });
  await expect.element(page.getByText("This repository is frozen")).toBeVisible();
  await page.getByRole("button", { name: "Unfreeze on the form" }).click();
  await expect.element(page.getByRole("tab", { name: "Edit" })).toHaveAttribute("aria-selected", "true");
  const freeze = page.getByRole("region", { name: "Freeze" });
  await expect.element(freeze.getByRole("switch", { name: /Freeze this repository/ })).toBeChecked();
  await expect.element(page.getByText("This repository is frozen")).toBeVisible();
  expect(page.getByRole("button", { name: "Unfreeze on the form" }).query()).toBeNull();
});

test("a workspace's Command with no root Manifest: listed alone, no Edit, and Run names the workspace", async () => {
  const onStartRun = vi.fn();
  await manifest({
    sheet: { state: "read", sheet: rootlessSheet() },
    setUp: false,
    runs: { runs: [WEB_DEV_RUN], unreadable: [] },
    onStartRun,
  });
  await expect.element(page.getByText("In apps/web.")).toBeVisible();
  expect(page.getByRole("tab", { name: "Edit" }).query()).toBeNull();
  await expect.element(page.getByText("dev in apps/web").first()).toBeVisible();
  await page.getByRole("button", { name: /^dev/ }).click();
  await page.getByRole("button", { name: "Run", exact: true }).click();
  expect(onStartRun).toHaveBeenCalledWith({ name: "dev", workspace: "apps/web" });
});
