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

/** Every drawn row, with the repository it names, if any, in either arrangement. */
async function rowsNaming(canvasElement: HTMLElement, named: RegExp | null) {
  const rows = [...canvasElement.querySelectorAll<HTMLElement>("[data-job-id]")];
  await expect(rows.length).toBeGreaterThan(0);
  for (const row of rows) {
    // The handle and the status keep their places beside it.
    await expect(row.querySelector(".armada-job-row__id")?.textContent).not.toBe("");
    await expect(row.querySelector(".armada-job-row__badge")).not.toBeNull();
    const values = [...row.querySelectorAll(".armada-job-row__field-value")].map((one) => one.textContent ?? "");
    await expect(values.some((value) => named?.test(value) ?? /^(armada|storefront)$/.test(value))).toBe(named !== null);
  }
}

/** Every Job, split between the two repositories. */
const acrossTwo = () =>
  boardJobs().map((job, at) => ({ ...job, owner_manifest_id: at % 2 === 0 ? "armada" : "storefront" }));

/** Two repositories served, on All: every row names its own by the picker's label, as a card and in the table. */
export const TwoRepositories: Story = {
  name: "Two repositories",
  args: { jobs: acrossTwo(), repositories: [ARMADA, STOREFRONT] },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByRole("button", { name: "All repositories" })).toBeInTheDocument();
    await rowsNaming(canvasElement, /^(armada|storefront)$/);
    await expect(within(canvasElement).getAllByText("Repository").length).toBeGreaterThan(0);
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Table" }));
    await expect(canvasElement.querySelector(".armada-active-jobs__columns")?.textContent).toContain("Repository");
    await rowsNaming(canvasElement, /^(armada|storefront)$/);
  },
};

/** Two served, one picked: the Board lists its Jobs alone, so no row or column names a repository. */
export const OnePicked: Story = {
  name: "One picked",
  args: { jobs: acrossTwo(), repositories: [ARMADA, STOREFRONT], picked: STOREFRONT.root },
  play: async ({ canvasElement }) => {
    await rowsNaming(canvasElement, null);
    await expect(canvasElement.querySelectorAll("[data-job-id]").length).toBeLessThan(acrossTwo().length);
    await expect(within(canvasElement).queryByText("Repository")).toBeNull();
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Table" }));
    await expect(canvasElement.querySelector(".armada-active-jobs__columns")?.textContent).not.toContain("Repository");
    await rowsNaming(canvasElement, null);
  },
};

/** One repository served, on All: the Board looks as it always did, with no column saying one name down every row. */
export const OneRepository: Story = {
  name: "One repository",
  play: async ({ canvasElement }) => {
    await rowsNaming(canvasElement, null);
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

/** The row a freeze holds, in both arrangements. */
function frozenRow(canvasElement: HTMLElement, handle: string): string {
  const row = Array.from(canvasElement.querySelectorAll<HTMLElement>("[data-job-id]")).find((one) =>
    one.textContent?.includes(handle),
  );
  return row?.textContent ?? "";
}

/**
 * `armada` frozen. The queued row reads frozen and names it; the row at review keeps its own
 * status and says nothing lands. Neither badge is invented: `frozen` is the registry's word.
 */
export const AFrozenRepository: Story = {
  name: "A frozen repository",
  args: {
    jobs: boardJobs().map((job) =>
      job.status === "queued"
        ? { ...job, queued_reason: "frozen", frozen_by: ["armada"] }
        : job.status === "awaiting_review"
          ? { ...job, frozen_by: ["armada"] }
          : job,
    ),
  },
  play: async ({ canvasElement }) => {
    for (const view of ["Cards", "Table"]) {
      if (view === "Table") await userEvent.click(within(canvasElement).getByRole("button", { name: "Table" }));
      await expect(frozenRow(canvasElement, "retire-the-legacy-poke-path")).toMatch(/frozen.*waits for armada/i);
      await expect(frozenRow(canvasElement, "split-the-settings-reducer")).toContain("lands after armada unfreezes");
    }
  },
};
