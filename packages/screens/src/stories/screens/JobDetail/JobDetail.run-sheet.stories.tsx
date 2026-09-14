import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, within } from "storybook/test";

import type { ServerState } from "@armada/protocol";
import { escalatedGateFailure, running } from "../../../fixtures/build/index";
import { JOB_ID } from "../../../fixtures/build/base";
import { propsFor } from "../../../fixtures/props";
import { JobDetailFrom } from "./JobDetail";
import { drawing } from "./story-helpers";

/** Job detail, split by group — #1044. Same `title` as the rest of this directory, so ids hold. */
const meta: Meta<typeof JobDetailFrom> = {
  title: "Screens/Job detail",
  component: JobDetailFrom,
  parameters: { layout: "fullscreen" },
};
export default meta;

type Story = StoryObj<typeof JobDetailFrom>;

// Journey 9's run sheet — #624. `r` opens it, one sheet replaces another, and
// a server's link never navigates.

/**
 * This repository's own `armada.yml` — Setup, Checks and Commands as they are
 * declared at the root of this repository — read as the Job's frozen
 * Manifest. A story with an empty rail draws what shipping the wire and
 * never loading it into a fixture looks like on screen, so this is the real
 * file rather than a name invented for the story.
 */
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

/** `r` opens the run sheet, nothing selected — Setup, Checks and Commands as
 * this repository's own `armada.yml` declares them. */
export const RunOpen: Story = {
  name: "Run sheet open",
  render: () => (
    <JobDetailFrom
      fixture={running()}
      on={{ rehearsal: { ...propsFor(running()).rehearsal, runSheet: ARMADA_RUN_SHEET_READ } }}
    />
  ),
  play: async ({ userEvent }) => {
    await userEvent.keyboard("r");
    const dialog = within(await within(document.body).findByRole("dialog", { name: "Run" }));
    await expect(dialog.findByText("bridge_test")).resolves.toBeVisible();
  },
};

/** Opening the run sheet while the log is open replaces it — one sheet at a time. */
export const RunReplacesLog: Story = {
  name: "Run sheet replaces the log",
  render: drawing(running),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: /Open the log/ }));
    await expect(within(document.body).findByRole("dialog", { name: "Activity log" })).resolves.toBeVisible();

    await userEvent.keyboard("r");
    await expect(within(document.body).findByRole("dialog", { name: "Run" })).resolves.toBeVisible();
    expect(within(document.body).queryByRole("dialog", { name: "Activity log" })).toBeNull();
  },
};

/** A server this Job started, serving. Its link hands the address to the
 * system browser rather than navigating — the design system's hard rule. */
const SERVING: ServerState = {
  id: "srv-storybook",
  name: "storybook",
  job_id: propsFor(running()).job.id,
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

const openServerLink = fn(async () => ({ ok: true }) as const);

/** The sheet's own reading of the Job's frozen Manifest, naming the Check
 * `escalatedGateFailure` failed — `cargo_nextest` — so selecting it has a row
 * to select. */
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

/** A refused Check's `Run it here` opens the run sheet with that Check selected. */
export const RunItHere: Story = {
  name: "Run it here selects the Check",
  render: () => (
    <JobDetailFrom
      fixture={escalatedGateFailure()}
      on={{ rehearsal: { ...propsFor(escalatedGateFailure()).rehearsal, runSheet: RUN_SHEET_READ } }}
    />
  ),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: "Run it here" }));
    const dialog = within(await within(document.body).findByRole("dialog", { name: "Run" }));
    await expect(dialog.findByRole("button", { current: true })).resolves.toHaveTextContent("cargo_nextest");
  },
};

/** `test` running, streaming its output as it prints. */
export const RunStreaming: Story = {
  name: "Running test, streaming",
  render: () => (
    <JobDetailFrom
      fixture={escalatedGateFailure()}
      on={{
        rehearsal: {
          ...propsFor(escalatedGateFailure()).rehearsal,
          runSheet: {
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
          runFollowed: {
            state: "following",
            jobId: JOB_ID,
            runId: "run-1",
            name: "cargo_nextest",
            path: ".armada/runs/run-1/output.log",
            fromLine: 1,
            lines: ["running 2034 tests", "test settings::selectors::visible_manifests_memoises ... FAIL"],
          },
        },
      }}
    />
  ),
  play: async ({ canvas, userEvent }) => {
    await userEvent.click(await canvas.findByRole("button", { name: "Run it here" }));
    await expect(
      within(await within(document.body).findByRole("dialog", { name: "Run" })).findByText(/FAIL/),
    ).resolves.toBeVisible();
  },
};

export const ServingRow: Story = {
  name: "Serving row, link opens the system browser",
  // Open, so the Serving row it holds is drawn — `whereOpen` is Fleet's own
  // preference and a story sets it directly, `#927`.
  render: () => (
    <JobDetailFrom
      fixture={running()}
      on={{
        whereOpen: true,
        rehearsal: {
          ...propsFor(running()).rehearsal,
          servers: { servers: [SERVING] },
          onOpenServerLink: openServerLink,
        },
      }}
    />
  ),
  play: async ({ canvas, userEvent }) => {
    openServerLink.mockClear();
    const link = await canvas.findByRole("button", { name: "Storybook" });
    await userEvent.click(link);
    await expect(openServerLink).toHaveBeenCalledWith(SERVING.id, SERVING.links[0]!.url);
  },
};
