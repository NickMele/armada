import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import type { JobSummary, RepositorySummary, WorkflowSummary } from "@armada/protocol";
import recorded from "../../../fixtures/boards/real-board.json";
import { repository } from "../../../fixtures/build/base";
import { boardJobs, boardWorkflows } from "../../../fixtures/build/board";
import { BoardFrom } from "./Board";

/**
 * The Board, drawn by the app's own `Shell`, `BoardActions` and `Jobs` from
 * rows in the shape `list_jobs` sends. Change the screen and this changes
 * with it; only the data is made up. `Screens/Job detail` is built the same
 * way.
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

/**
 * `list-keyboard.ts`'s own coverage: proves the extraction into a shared mechanism left the Board's
 * own keyboard unchanged. `Screens/Overview lists`' "Keyboard moves the cursor and acts on it" is
 * the same story on the other screen the mechanism now serves.
 */
export const KeyboardMovesTheCursorAndActs: Story = {
  name: "Keyboard moves the cursor and acts on it",
  args: { onOpen: fn(), onKill: fn() },
  play: async ({ args, canvasElement }) => {
    const rows = [...canvasElement.querySelectorAll<HTMLElement>("[data-job-id]")];
    await expect(rows.length).toBeGreaterThan(3);
    rows[0]!.focus();

    await userEvent.keyboard("jj");
    await expect(rows[2]).toHaveFocus();
    await userEvent.keyboard("k");
    await expect(rows[1]).toHaveFocus();

    await userEvent.keyboard("{Enter}");
    await expect(args.onOpen).toHaveBeenCalledWith(rows[1]!.dataset.jobId);

    rows[0]!.focus();
    await userEvent.keyboard("x");
    await expect(args.onKill).toHaveBeenCalledWith(rows[0]!.dataset.jobId);
  },
};

/**
 * `n` — `new_job`'s scope is `anywhere`, so the Board answers it with no row
 * under the cursor at all. `actions.toml`'s own reading of the key, drawn
 * from `keys.ts`'s `compose` act and answered in `Jobs.tsx`.
 */
export const DispatchKeyOpensTheComposer: Story = {
  name: "Dispatch key opens the composer",
  args: { onCompose: fn() },
  play: async ({ args, canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.keyboard("n");
    await expect(args.onCompose).toHaveBeenCalledTimes(1);

    // Typing "n" into the search field is typing, not a shortcut — `holdsText`
    // in `keys.ts` is the whole of that rule, and every single-key press goes
    // through it.
    const search = canvas.getByRole("searchbox", { name: "Search every job" });
    await userEvent.type(search, "n");
    await expect(args.onCompose).toHaveBeenCalledTimes(1);
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

/** The row itself, found by its handle — `frozenRow`'s own query, kept as an element rather than flattened to text. */
function rowNaming(canvasElement: HTMLElement, handle: string): HTMLElement {
  const row = Array.from(canvasElement.querySelectorAll<HTMLElement>("[data-job-id]")).find((one) =>
    one.textContent?.includes(handle),
  );
  if (row === undefined) throw new Error(`no row named ${handle} on this board`);
  return row;
}

/**
 * Overview 28 (#1092): Recently ended's own split act. `killed` and
 * `completed_failed` lead with Redispatch, Clear in the caret; `rejected`
 * never ran, so `crates/fleet/src/redispatch.rs` refuses it by name and the
 * row keeps the plain verb instead — offering an act guaranteed to fail is
 * worse than the section's usual one.
 */
export const RecentlyEnded: Story = {
  name: "Recently ended — redispatch, and the one row that cannot",
  play: async ({ canvasElement }) => {
    const redispatchable = within(canvasElement).getAllByRole("button", { name: "Redispatch as a new job" });
    await expect(redispatchable.length).toBe(2);

    const rejected = rowNaming(canvasElement, "11-widen-the-search-index");
    await expect(within(rejected).getByRole("button", { name: "Open" })).toBeVisible();
    await expect(within(rejected).queryByRole("button", { name: "Redispatch as a new job" })).toBeNull();

    // Clear is in every Recently ended row's caret, the rejected one included.
    await userEvent.click(within(rejected).getByRole("button", { name: /More for/ }));
    await expect(within(rejected).getByRole("menuitem", { name: "Clear" })).toBeVisible();
  },
};

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
