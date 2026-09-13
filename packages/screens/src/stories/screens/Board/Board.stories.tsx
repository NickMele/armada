import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, within } from "storybook/test";
import type { JobSummary, RepositorySummary, WorkflowSummary } from "@armada/protocol";
import recorded from "../../../fixtures/boards/real-board.json";
import { repository } from "../../../fixtures/build/base";
import { boardJobs, boardWorkflows } from "../../../fixtures/build/board";
import { BoardFrom } from "./Board";

/**
 * The Board, drawn by the app's own `Shell`, `headOf` and `Jobs` from rows in
 * the shape `list_jobs` sends. Change the screen and this changes with it;
 * only the data is made up. `Screens/Job detail` is built the same way.
 */
const meta = {
  title: "Screens/Board",
  component: BoardFrom,
  parameters: { layout: "fullscreen" },
  args: { jobs: boardJobs(), workflows: boardWorkflows() },
} satisfies Meta<typeof BoardFrom>;

export default meta;
type Story = StoryObj<typeof meta>;

/** One Job in every state the Board lists. */
export const EveryState: Story = { name: "Every state" };

/**
 * A real Board, as Fleet served it on 11 Sep 2026, recorded with
 * `scripts/record-job.mjs --board`. The rows above are built to reach every
 * state; this one is what a day's work actually left.
 */
export const Recorded: Story = {
  name: "Recorded",
  args: {
    jobs: recorded.jobs as unknown as JobSummary[],
    workflows: recorded.workflows as unknown as WorkflowSummary[],
    // A few minutes after the running Job started, when this was recorded.
    now: Date.parse("2026-09-11T17:43:43Z"),
  },
};

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const STOREFRONT: RepositorySummary = {
  root: "/Users/user/code/storefront",
  records_root: "/records/storefront",
  manifest: { ...repository().manifest!, id: "storefront", repository: "storefront", path: "storefront/armada.yml" },
};

/** Two repositories served: the Board lists the picked one's Jobs, so no row or column names a repository. */
export const TwoRepositories: Story = {
  name: "Two repositories",
  args: { repositories: [ARMADA, STOREFRONT] },
  play: async ({ canvasElement }) => {
    await expect(canvasElement.querySelectorAll("[data-job-id]").length).toBeGreaterThan(0);
    await expect(within(canvasElement).queryByText("Repository")).toBeNull();
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Table" }));
    await expect(canvasElement.querySelector(".armada-active-jobs__columns")?.textContent).not.toContain("Repository");
  },
};

/**
 * `#898`. Job 2's plan is partway done, job 4's is complete but for a dropped
 * task, and every other row carries no plan at all — the task field is drawn
 * only where `JobSummary.tasks` is present.
 */
export const TaskBars: Story = {
  name: "Task bars — partway, complete, and none",
  args: {
    jobs: boardJobs().map((job, at) => {
      if (at === 1) return { ...job, tasks: { done: 1, working: 1, open: 1, dropped: 0 } };
      if (at === 3) return { ...job, tasks: { done: 3, working: 0, open: 0, dropped: 1 } };
      return job;
    }),
  },
  play: async ({ canvasElement }) => {
    // Scoped to the row's own figure, not the bar's tooltip echoing the same
    // text — `getAllByText` does not know one is hidden.
    const figures = [...canvasElement.querySelectorAll(".armada-row-step")]
      .map((el) => el.textContent ?? "")
      .filter((text) => /^\d+ of \d+ tasks$/.test(text));
    await expect(figures.length).toBe(2);
  },
};

/** Fleet holds no Jobs for this repository. */
export const Empty: Story = { name: "Empty", args: { jobs: [] } };

/** Fleet is not running, so the Board has nothing it can trust. */
export const FleetNotRunning: Story = {
  name: "Fleet not running",
  args: {
    jobs: [],
    connection: {
      state: "not_running",
      absence: {
        why: "no_runtime_file",
        path: "/Users/user/Library/Application Support/Armada/fleet.runtime.json",
      },
    },
  },
};
