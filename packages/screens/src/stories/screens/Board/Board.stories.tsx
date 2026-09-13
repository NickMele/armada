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

/** Two repositories served: every row names its own by the picker's label, as a card and in the table. */
export const TwoRepositories: Story = {
  name: "Two repositories",
  args: {
    jobs: boardJobs().map((job, at) => ({ ...job, owner_manifest_id: at % 2 === 0 ? "armada" : "storefront" })),
    repositories: [ARMADA, STOREFRONT],
  },
  play: async ({ canvasElement }) => {
    await rowsNaming(canvasElement, /^(armada|storefront)$/);
    await expect(within(canvasElement).getAllByText("Repository").length).toBeGreaterThan(0);
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Table" }));
    await expect(canvasElement.querySelector(".armada-active-jobs__columns")?.textContent).toContain("Repository");
    await rowsNaming(canvasElement, /^(armada|storefront)$/);
  },
};

/** One repository served: the Board looks as it always did, with no column saying the same name down every row. */
export const OneRepository: Story = {
  name: "One repository",
  play: async ({ canvasElement }) => {
    await rowsNaming(canvasElement, null);
    await expect(within(canvasElement).queryByText("Repository")).toBeNull();
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Table" }));
    await expect(canvasElement.querySelector(".armada-active-jobs__columns")?.textContent).not.toContain("Repository");
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
