// Overview and Settings, through `App`. Moved here from the `Screens/Overview
// lists`, `Screens/Overview summary`, `Screens/Overview surface` and
// `Screens/Settings surface` stories, which stood each in a shell of their own
// rather than the one `App` draws — #1224.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { FleetLimits, JobSummary, RepositorySummary } from "@armada/protocol";
import { job, repository } from "@armada/screens/src/fixtures/build/base";
import { boardJobs, boardWorkflows } from "@armada/screens/src/fixtures/build/board";

import { onBoard } from "./scenario";
import type { Scenario } from "./scenario";
import { mount, rows, unmountAfterEach } from "./testing";

unmountAfterEach();

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const STOREFRONT: RepositorySummary = {
  root: "/Users/user/code/storefront",
  records_root: "/records/storefront",
  manifest: { ...repository().manifest!, id: "storefront", repository: "storefront", path: "storefront/armada.yml" },
};

/** One Job in every state the Board names, and one status this build's registry does not know. */
const JOBS = (): JobSummary[] => [
  ...boardJobs(),
  job("not_a_status_the_registry_has", { id: "01M2C1TJ8G00UNKNOWNSTATUS00", handle: "11-unknown-status" }),
];

/** Two running Jobs, one with a plan and one without — the owner's report against the Running panel. */
const RUNNING_ONE_WITH_A_PLAN = (): JobSummary[] => [
  job("running", {
    id: "01M2C1TJ8G00RUNNINGWITHTASK",
    handle: "16-fix-801-unanswered-permission-ask-holds-dr",
    title: "Fix 801: unanswered permission ask holds Drone slot, 2nd dispatch",
    workflow_id: "implement",
    owner_manifest_id: "armada",
    current_step_id: "fix",
    started_at: new Date(Date.now() - 476_000).toISOString(),
    tasks: { done: 0, working: 0, open: 6, dropped: 0 },
  }),
  job("running", {
    id: "01M2C1TJ8G00RUNNINGNOPLAN00",
    handle: "17-job-details-view-shows-queued-while-fleet-r",
    title: 'Job details view shows "queued" while Fleet reports job as running',
    workflow_id: "plan",
    owner_manifest_id: "armada",
    current_step_id: "plan",
    started_at: new Date(Date.now() - 138_000).toISOString(),
  }),
];

/** Overview, where `App` opens, on these rows. */
async function overview(scenario: Scenario): Promise<void> {
  mount(scenario);
  await expect.element(page.getByRole("heading", { name: "Running" }).first()).toBeVisible();
}

const onOverview = (jobs: JobSummary[], options: Parameters<typeof onBoard>[1] = {}) =>
  onBoard(jobs, { workflows: boardWorkflows(), ...options });

/** Fleet was connected and has gone: what Bridge holds is what it last received. */
function unreachable(scenario: Scenario): Scenario {
  const connection = scenario.state.connection;
  if (connection.state !== "connected") throw new Error("unreachable from a connected scenario");
  return {
    ...scenario,
    state: {
      ...scenario.state,
      connection: { state: "unreachable", fleet: connection.fleet, detail: "Fleet unreachable", sinceMs: Date.now() },
    },
  };
}

test("every section draws, and a status the registry does not know is named beneath", async () => {
  await overview(onOverview(JOBS()));
  await expect.element(page.getByRole("heading", { name: "Needs you" })).toBeVisible();
  await expect.element(page.getByRole("heading", { name: "Queued" })).toBeVisible();
  await expect.element(page.getByText(/not_a_status_the_registry_has/)).toBeVisible();
  expect(page.getByRole("heading", { name: "Done" }).query()).toBeNull();
});

test("no jobs: a card with Dispatch on it, and every summary count reads zero", async () => {
  mount(onOverview([]));
  // Still named "Overview" to assistive tech, with the card inside it. #1262.
  const empty = page.getByRole("list", { name: "Overview" });
  await expect.element(empty.getByText("No jobs.", { exact: true })).toBeVisible();
  await expect.element(empty.getByText("Propose one.", { exact: true })).toBeVisible();
  await expect.element(empty.getByRole("button", { name: "Dispatch", exact: true })).toBeVisible();
  await expect.element(page.getByRole("button", { name: "0 Needs you" })).toBeVisible();
});

test("two repositories on All: an Overview row names its own", async () => {
  await overview(onOverview(JOBS().map((row, at) => ({ ...row, owner_manifest_id: at % 2 === 0 ? "armada" : "storefront" })), {
    repositories: [ARMADA, STOREFRONT],
  }));
  await expect
    .poll(() =>
      [...document.querySelectorAll(".armada-job-row__field-value")].some((one) =>
        /^(armada|storefront)$/.test(one.textContent ?? ""),
      ),
    )
    .toBe(true);
});

test("Fleet unreachable: the rows held from before stay, Needs you included", async () => {
  await overview(unreachable(onOverview(JOBS())));
  await expect.element(page.getByRole("heading", { name: "Needs you" })).toBeVisible();
  expect(rows().length).toBeGreaterThan(0);
});

test("Fleet unreachable with nothing held says so flatly", async () => {
  mount(unreachable(onOverview([])));
  await expect.element(page.getByText("Fleet is not connected, so there is nothing to show.")).toBeVisible();
});

test("the Running panel beside Helm's dock keeps each row's facts and its action", async () => {
  await overview(onOverview(RUNNING_ONE_WITH_A_PLAN(), { repositories: [ARMADA, STOREFRONT] }));
  await expect.element(page.getByRole("complementary", { name: "Helm" })).toBeVisible();
  // In the DOM, not asserted visible: under 900px a card row gives up Run time,
  // Repository and Tasks so its action stays reachable, by design —
  // `JobRowStacked.narrow.css`. Beside the dock the Running panel is that narrow.
  await expect.poll(() => document.querySelector('[role="img"][aria-label="0 of 6 tasks"]')).not.toBeNull();
  await expect.element(page.getByRole("option", { name: /unanswered permission ask/ })).toBeVisible();
  await expect.element(page.getByRole("option", { name: /shows "queued"/ })).toBeVisible();
  await expect.element(page.getByRole("button", { name: /Redirect/ }).first()).toBeVisible();
});

test("j and k move Overview's cursor across sections, Enter opens, x asks to kill", async () => {
  await overview(onOverview(JOBS()));
  const drawn = rows();
  expect(drawn.length).toBeGreaterThan(3);
  drawn[0]!.focus();

  await userEvent.keyboard("jj");
  await expect.element(drawn[2]!).toHaveFocus();
  await userEvent.keyboard("k");
  await expect.element(drawn[1]!).toHaveFocus();

  await userEvent.keyboard("x");
  await expect.element(page.getByRole("dialog")).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.poll(() => page.getByRole("dialog").query()).toBeNull();

  const handle = drawn[1]!.querySelector(".armada-job-row__id")?.textContent ?? "";
  drawn[1]!.focus();
  await userEvent.keyboard("{Enter}");
  await expect.poll(() => rows().length).toBe(0);
  await expect.element(page.getByText(handle, { exact: true }).first()).toBeVisible();
});

test("the focused row is Overview's cursor, and Helm's footer names it", async () => {
  await overview(onOverview(JOBS()));
  await expect.element(page.getByText("Overview", { exact: true }).last()).toBeVisible();
  rows()[0]!.focus();
  await expect.element(page.getByText(/^Overview · cursor on Job \d+$/)).toBeVisible();
});

test("n opens the composer from Overview", async () => {
  await overview(onOverview(JOBS()));
  await userEvent.keyboard("n");
  await expect.element(page.getByText("Pick the repository this Job is for")).toBeVisible();
});

test("Running folds and reopens, and folding it leaves the other panels open", async () => {
  await overview(onOverview(JOBS()));
  await page.getByRole("button", { name: "Collapse Running" }).click();
  await expect.element(page.getByRole("button", { name: "Collapse Needs you" })).toBeVisible();
  await page.getByRole("button", { name: "Expand Running" }).click();
  await expect.element(page.getByRole("button", { name: "Collapse Running" })).toBeVisible();
});

test("a summary tile opens its panel", async () => {
  await overview(onOverview(JOBS()));
  await page.getByRole("button", { name: "Collapse Queued" }).click();
  await expect.element(page.getByRole("button", { name: "Expand Queued" })).toBeVisible();
  await page.getByRole("button", { name: /\d+ Queued/ }).click();
  await expect.element(page.getByRole("button", { name: "Collapse Queued" })).toBeVisible();
});

/** Fleet's four limits, as `get_limits` answers them. */
const LIMITS: FleetLimits = {
  concurrency: 2,
  memory_spare_percent: 15,
  disk_floor_gib: 10,
  checks_at_once: 4,
  shipped: { concurrency: 2, memory_spare_percent: 15, disk_floor_gib: 10, checks_at_once: 4 },
};

test("Settings draws Fleet's limits and this machine's settings beside Helm's dock", async () => {
  const scenario = onOverview(JOBS());
  mount({ ...scenario, state: { ...scenario.state, limits: LIMITS } });
  await page.getByRole("button", { name: "Settings", exact: true }).click();
  await expect.element(page.getByRole("heading", { name: "Fleet" })).toBeVisible();
  await expect.element(page.getByRole("heading", { name: "This machine" })).toBeVisible();
  await expect.element(page.getByRole("complementary", { name: "Helm" })).toBeVisible();
});
