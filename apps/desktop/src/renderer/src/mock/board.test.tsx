// The Board, through `App`: what it lists, how it names a repository, and what
// its keys do. Moved here from `Screens/Board`'s stories, which drew the Board
// from a copy of `App`'s wiring — #1224.

import { expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { JobSummary, RepositorySummary } from "@armada/protocol";
import { repository } from "@armada/screens/src/fixtures/build/base";
import { boardJobs, boardWorkflows } from "@armada/screens/src/fixtures/build/board";

import { onBoard } from "./scenario";
import { mount, openBoard, rows, unmountAfterEach } from "./testing";

unmountAfterEach();

const ARMADA: RepositorySummary = { ...repository(), manifest: { ...repository().manifest!, id: "armada" } };
const STOREFRONT: RepositorySummary = {
  root: "/Users/user/code/storefront",
  records_root: "/records/storefront",
  manifest: { ...repository().manifest!, id: "storefront", repository: "storefront", path: "storefront/armada.yml" },
};

/** Every Job, split between the two repositories. */
const acrossTwo = (): JobSummary[] =>
  boardJobs().map((job, at) => ({ ...job, owner_manifest_id: at % 2 === 0 ? "armada" : "storefront" }));

/** The Board on these rows, opened from the rail. */
async function board(jobs: JobSummary[], options: Parameters<typeof onBoard>[1] = {}): Promise<void> {
  mount(onBoard(jobs, { workflows: boardWorkflows(), ...options }));
  await openBoard();
}

/** Every drawn row names a repository matching `named`, or none where `named` is null. Handle and badge stay. */
function expectRowsNaming(named: RegExp | null): void {
  expect(rows().length).toBeGreaterThan(0);
  for (const row of rows()) {
    expect(row.querySelector(".armada-job-row__id")?.textContent).not.toBe("");
    expect(row.querySelector(".armada-job-row__badge")).not.toBeNull();
    const values = [...row.querySelectorAll(".armada-job-row__field-value")].map((one) => one.textContent ?? "");
    expect(values.some((value) => named?.test(value) ?? /^(armada|storefront)$/.test(value))).toBe(named !== null);
  }
}

const columns = () => document.querySelector(".armada-active-jobs__columns")?.textContent ?? "";

/** The row whose text holds this handle. */
function rowNaming(handle: string): HTMLElement {
  const row = rows().find((one) => one.textContent?.includes(handle));
  if (row === undefined) throw new Error(`no row named ${handle} on this board`);
  return row;
}

test("two repositories on All: every row names its own, as a card and in the table", async () => {
  await board(acrossTwo(), { repositories: [ARMADA, STOREFRONT] });
  await expect.element(page.getByRole("button", { name: "All repositories" }).first()).toBeVisible();
  expectRowsNaming(/^(armada|storefront)$/);
  expect(page.getByText("Repository").elements().length).toBeGreaterThan(0);
  await page.getByRole("button", { name: "Table" }).click();
  await expect.poll(columns).toContain("Repository");
  expectRowsNaming(/^(armada|storefront)$/);
});

test("two served, one picked: only its Jobs, and nothing names a repository", async () => {
  await board(acrossTwo(), { repositories: [ARMADA, STOREFRONT], picked: STOREFRONT.root });
  expectRowsNaming(null);
  expect(rows().length).toBeLessThan(acrossTwo().length);
  expect(page.getByText("Repository", { exact: true }).query()).toBeNull();
  await page.getByRole("button", { name: "Table" }).click();
  await expect.poll(() => document.querySelector(".armada-active-jobs__columns")).not.toBeNull();
  expect(columns()).not.toContain("Repository");
  expectRowsNaming(null);
});

test("one repository on All: no row or column names it", async () => {
  await board(boardJobs());
  expectRowsNaming(null);
  expect(page.getByText("Repository", { exact: true }).query()).toBeNull();
  await page.getByRole("button", { name: "Table" }).click();
  await expect.poll(() => document.querySelector(".armada-active-jobs__columns")).not.toBeNull();
  expect(columns()).not.toContain("Repository");
});

test("a task figure is drawn only on a row whose Job carries a plan", async () => {
  await board(
    boardJobs().map((job, at) => {
      if (at === 1) return { ...job, tasks: { done: 1, working: 1, open: 1, dropped: 0 } };
      if (at === 3) return { ...job, tasks: { done: 3, working: 0, open: 0, dropped: 1 } };
      return job;
    }),
  );
  // The row's own figure, not the bar's tooltip echoing the same text.
  const figures = () =>
    [...document.querySelectorAll(".armada-row-step")]
      .map((el) => el.textContent ?? "")
      .filter((text) => /^\d+ of \d+ tasks$/.test(text));
  await expect.poll(() => figures().length).toBe(2);
});

test("j and k move the cursor, Enter opens the row under it, and x asks to kill it", async () => {
  await board(boardJobs());
  const drawn = rows();
  expect(drawn.length).toBeGreaterThan(3);
  drawn[0]!.focus();

  await userEvent.keyboard("jj");
  await expect.element(drawn[2]!).toHaveFocus();
  await userEvent.keyboard("k");
  await expect.element(drawn[1]!).toHaveFocus();

  // x on the row under the cursor asks, through the dialog every kill goes through.
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

test("n opens the composer from the Board, and typing n into search does not", async () => {
  await board(boardJobs());
  const search = page.getByRole("searchbox", { name: "Search every job" });
  await userEvent.type(search, "n");
  expect(page.getByText("Pick the repository this Job is for").query()).toBeNull();
  expect(rows().length).toBeGreaterThan(0);

  (document.activeElement as HTMLElement | null)?.blur();
  await userEvent.keyboard("n");
  await expect.element(page.getByText("Pick the repository this Job is for")).toBeVisible();
});

test("Recently ended offers Redispatch, except on the rejected row, and Clear on every caret", async () => {
  await board(boardJobs());
  await expect.poll(() => page.getByRole("button", { name: "Redispatch as a new job" }).elements().length).toBe(2);

  const rejected = page.elementLocator(rowNaming("11-widen-the-search-index"));
  await expect.element(rejected.getByRole("button", { name: "Open" })).toBeVisible();
  expect(rejected.getByRole("button", { name: "Redispatch as a new job" }).query()).toBeNull();

  await rejected.getByRole("button", { name: /More for/ }).click();
  await expect.element(page.getByRole("menuitem", { name: "Clear" })).toBeVisible();
});

test("a frozen repository: the queued row waits for it, the row at review says it lands after", async () => {
  await board(
    boardJobs().map((job) =>
      job.status === "queued"
        ? { ...job, queued_reason: "frozen", frozen_by: ["armada"] }
        : job.status === "awaiting_review"
          ? { ...job, frozen_by: ["armada"] }
          : job,
    ),
  );
  for (const view of ["Cards", "Table"]) {
    if (view === "Table") await page.getByRole("button", { name: "Table" }).click();
    await expect.poll(() => rowNaming("retire-the-legacy-poke-path").textContent).toMatch(/frozen.*waits for armada/i);
    await expect.poll(() => rowNaming("split-the-settings-reducer").textContent).toContain("lands after armada unfreezes");
  }
});

/** The loops drawing under this keyframe right now: what moves on screen, whatever element carries it. */
const looping = (keyframes: string): number =>
  document.getAnimations().filter((one) => (one as CSSAnimation).animationName === keyframes).length;

test("every running row's badge pulses with the cursor on none of them", async () => {
  // Three running, so a pulse that follows the cursor cannot pass by landing on the only one.
  await board(boardJobs().map((job) => (job.status === "queued" ? { ...job, status: "running" } : job)));
  (document.activeElement as HTMLElement | null)?.blur();
  const running = () => rows().filter((row) => row.querySelector(".armada-job-row__badge")?.textContent?.toLowerCase() === "running");
  await expect.poll(() => running().length).toBe(3);
  await expect.poll(() => looping("armada-badge-pulse")).toBe(3);
});
