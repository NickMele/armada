// Job detail's sheets and its run sheet, through `App`: what opens one, what
// replaces one, and what a server's link sends. Moved here from the `Screens/Job
// detail` stories' sheets and run-sheet groups — #1224.

import { expect, test, vi } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { ServerState } from "@armada/protocol";
import { escalatedGateFailure, gateChecksStreaming, running } from "@armada/screens/src/fixtures/build/index";
import { JOB_ID } from "@armada/screens/src/fixtures/build/base";
import type { JobFixture } from "@armada/screens/src/fixtures/fixture";

import type { BridgeApi } from "../../../shared/api";
import type { BridgeState } from "../../../shared/bridge";
import type { FleetHandle } from "./scenario";
import { onJob } from "./scenario";
import { mount, unmountAfterEach } from "./testing";

unmountAfterEach();

/** App with this Job open. `sheet` is the run sheet Fleet reads for it, and `also` more state beside. */
async function opened(
  fixture: JobFixture,
  {
    sheet,
    followed,
    also = {},
    whereOpen = false,
  }: { sheet?: BridgeState["runSheet"]; followed?: BridgeState["runFollowed"]; also?: Partial<BridgeState>; whereOpen?: boolean } = {},
): Promise<BridgeApi> {
  const scenario = onJob(fixture, { whereOpen });
  const behaves = (fleet: FleetHandle): Partial<BridgeApi> => ({
    ...(sheet === undefined
      ? {}
      : { watchRunSheet: async (jobId) => fleet.publish({ runSheet: jobId === null ? { state: "none" } : sheet }) }),
    ...(followed === undefined
      ? {}
      : { observeRun: async (_jobId, runId) => fleet.publish({ runFollowed: runId === null ? { state: "none" } : followed }) }),
  });
  const app = mount({ ...scenario, state: { ...scenario.state, ...also }, behaves });
  await expect.element(page.getByText(fixture.job.handle, { exact: true }).first()).toBeVisible();
  return app.api;
}

const dialog = (name: string) => page.getByRole("dialog", { name });

test("the log opens from its chapter's own control, and the control leaves the chapter", async () => {
  await opened(running());
  await page.getByRole("button", { name: /Open the log/ }).click();
  await expect.element(dialog("Activity log")).toBeVisible();
  await expect.poll(() => page.getByRole("button", { name: /Open the log/ }).query()).toBeNull();
});

/** The loops drawing under this keyframe right now: what moves on screen, whatever element carries it. */
const looping = (keyframes: string): number =>
  document.getAnimations().filter((one) => (one as CSSAnimation).animationName === keyframes).length;

test("the rail's current step keeps pulsing behind an open sheet", async () => {
  await opened(running());
  await expect.poll(() => looping("armada-step-mark-pulse")).toBeGreaterThan(0);
  // The same loops before and after: the chapter's live step shares the keyframe, so only a count says the rail's stayed.
  const before = looping("armada-step-mark-pulse");
  await page.getByRole("button", { name: /Open the log/ }).click();
  await expect.element(dialog("Activity log")).toBeVisible();
  expect(looping("armada-step-mark-pulse")).toBe(before);
});

test("the Job's patch opens from the Produced chapter", async () => {
  await opened(running());
  await page.getByRole("button", { name: /Open the diff/ }).click();
  await expect.element(page.getByRole("dialog")).toBeVisible();
});

test("the log opens on a Job a failed Check stopped", async () => {
  await opened(escalatedGateFailure());
  await page.getByRole("button", { name: /Open the log/ }).click();
  await expect.element(dialog("Activity log")).toBeVisible();
});

test("the failed Check's output opens from the header's act, in the editor rather than a sheet", async () => {
  const api = await opened(escalatedGateFailure());
  const openArtifact = vi.spyOn(api, "openArtifact");
  await page.getByRole("button", { name: /Open the output/ }).first().click();
  await expect.poll(() => openArtifact.mock.calls.length).toBe(1);
  expect(openArtifact.mock.calls[0]![0]).toBe(JOB_ID);
});

test("a kept Check's row opens the output in a sheet, and Escape closes it", async () => {
  await opened(escalatedGateFailure());
  expect(page.getByRole("dialog").query()).toBeNull();
  // The gate card names the same file as a chip to copy; the Checks row is the one that opens the sheet.
  await page.getByRole("button", { name: "regression_verify.3.cargo_nextest.log", pressed: false }).last().click();
  const output = dialog("Console output");
  await expect.element(output.getByText(/visible_manifests_memoises/).first()).toBeVisible();
  await expect.element(output.getByText("cargo_nextest — output")).toBeVisible();
  await userEvent.keyboard("{Escape}");
  await expect.poll(() => page.getByRole("dialog").query()).toBeNull();
});

test("a running Check's row opens the sheet on the same press, and asks main to follow the log", async () => {
  const api = await opened(gateChecksStreaming());
  const follow = vi.spyOn(api, "followCheckOutput");
  await page.getByRole("button", { name: "regression_verify.1.cargo_nextest.live.log", pressed: false }).click();
  await expect.element(dialog("Console output").getByText(/Opening this Check.s log/)).toBeVisible();
  await expect.poll(() => follow.mock.calls.some(([jobId, kept]) => jobId === JOB_ID && kept !== null)).toBe(true);
});

test("Pulse's Details opens the full reading", async () => {
  await opened(running());
  await page.getByRole("button", { name: /^Details/ }).click();
  await expect.element(page.getByRole("dialog")).toBeVisible();
});

/** This repository's own `armada.yml`, read as the Job's frozen Manifest. */
const ARMADA_RUN_SHEET_READ = {
  state: "read" as const,
  jobId: JOB_ID,
  sheet: {
    job_id: JOB_ID,
    setup: [
      { name: "bootstrap", run: "pnpm install --frozen-lockfile", narrows: false, requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "browsers", run: "pnpm -C packages/components exec playwright install chromium --only-shell", narrows: false, requires: [], expect_exit_code: 0, destructive: false, frozen: true },
    ],
    checks: [
      { name: "build", run: "cargo build --workspace --locked", narrows: true, narrow_run: "cargo build --locked -p screens", requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "test", run: "cargo nextest run --workspace --exclude acceptance", narrows: true, narrow_run: "cargo nextest run -p screens", requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "typecheck", run: "pnpm typecheck", narrows: false, requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "bridge_build", run: "pnpm -C apps/desktop build", narrows: false, requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "storybook", run: "pnpm -C packages/components build-storybook", narrows: false, requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "bridge_test", run: "pnpm bridge-test", narrows: false, requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "format", run: "cargo fmt --all --check", narrows: true, narrow_run: "rustfmt --check --edition 2021 packages/screens/src/JobDetail.tsx", requires: ["fmt"], expect_exit_code: 0, destructive: false, frozen: true },
    ],
    commands: [
      { name: "fmt", run: "cargo fmt --all", narrows: false, requires: [], expect_exit_code: 0, destructive: false, frozen: true },
      { name: "gate", run: "cargo xtask verify-foundations", narrows: false, requires: [], expect_exit_code: 0, destructive: true, frozen: true },
    ],
    manifest_edited_at: "2026-09-08T16:40:00Z",
    worktree_on_disk: true,
    worktree_differs: false,
    drone_working: true,
  },
};

const RUN_SHEET_READ = {
  state: "read" as const,
  jobId: JOB_ID,
  sheet: {
    job_id: JOB_ID,
    setup: [],
    checks: [
      {
        name: "cargo_nextest",
        run: "cargo nextest run --workspace",
        narrows: false,
        requires: [],
        expect_exit_code: 0,
        destructive: false,
        frozen: true,
      },
    ],
    commands: [],
    worktree_on_disk: true,
    worktree_differs: false,
    drone_working: false,
  },
};

const SERVING: ServerState = {
  id: "srv-storybook",
  name: "storybook",
  job_id: JOB_ID,
  phase: "serving",
  serve: "pnpm storybook",
  ports: [{ name: "PORT", port: 41207 }],
  links: [{ url: "http://localhost:41207", name: "Storybook" }],
  started_by: "person",
  started_at: "2026-09-10T18:00:00Z",
  serving_since: "2026-09-10T18:00:05Z",
  stopped: false,
  log: "run/storybook.log",
};

test("r opens the run sheet on this repository's own Setup, Checks and Commands", async () => {
  await opened(running(), { sheet: ARMADA_RUN_SHEET_READ });
  await userEvent.keyboard("r");
  await expect.element(dialog("Run").getByText("bridge_test")).toBeVisible();
});

test("the run sheet replaces the log: one sheet at a time", async () => {
  await opened(running());
  await page.getByRole("button", { name: /Open the log/ }).click();
  await expect.element(dialog("Activity log")).toBeVisible();
  await userEvent.keyboard("r");
  await expect.element(dialog("Run")).toBeVisible();
  expect(dialog("Activity log").query()).toBeNull();
});

test("a refused Check's Run it here opens the run sheet with that Check selected", async () => {
  await opened(escalatedGateFailure(), { sheet: RUN_SHEET_READ });
  await page.getByRole("button", { name: "Run it here" }).click();
  await expect.element(dialog("Run")).toBeVisible();
  await expect.poll(() => dialog("Run").element().querySelector('button[aria-current="true"]')?.textContent).toContain("cargo_nextest");
});

test("a Check running from the sheet streams its output as it prints", async () => {
  await opened(escalatedGateFailure(), {
    sheet: {
      ...RUN_SHEET_READ,
      sheet: {
        ...RUN_SHEET_READ.sheet,
        running: {
          id: "run-1",
          job_id: JOB_ID,
          name: "cargo_nextest",
          command: "cargo nextest run --workspace",
          narrowed: false,
          started_at: "2026-09-11T14:05:00Z",
        },
      },
    },
    followed: {
        state: "following",
        jobId: JOB_ID,
        runId: "run-1",
        name: "cargo_nextest",
        path: ".armada/runs/run-1/output.log",
        fromLine: 1,
        lines: ["running 2034 tests", "test settings::selectors::visible_manifests_memoises ... FAIL"],
    },
  });
  await page.getByRole("button", { name: "Run it here" }).click();
  await expect.element(dialog("Run").getByText(/FAIL/)).toBeVisible();
});

test("a serving row's link hands the address to the system browser rather than navigating", async () => {
  const api = await opened(running(), { also: { servers: { servers: [SERVING] } }, whereOpen: true });
  const openServerLink = vi.spyOn(api, "openServerLink");
  await page.getByRole("button", { name: "Storybook", exact: true }).click();
  await expect.poll(() => openServerLink.mock.calls.length).toBe(1);
  expect(openServerLink).toHaveBeenCalledWith(SERVING.id, SERVING.links[0]!.url);
});
