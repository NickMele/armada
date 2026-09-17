// A changed row decays on the Board itself: `recent.ts` wired through `Jobs` and `Row`.

import type { JobSummary } from "@armada/protocol";
import { afterEach, expect, test } from "vitest";

import { Jobs, type JobsProps } from "./Jobs";
import { job, workflow } from "./fixtures/build/base";
import { mount, rerender, unmount } from "./mounted";

afterEach(unmount);

const noop = () => {};
const AGO = /· \d+s ago$/;

function board(jobs: readonly JobSummary[]): React.ReactElement {
  const props: JobsProps = {
    jobs,
    stale: false,
    now: Date.parse("2026-09-10T21:00:00Z"),
    workflows: [workflow()],
    disconnected: null,
    selected: null,
    onOpen: noop,
    onKill: noop,
    onRedispatch: noop,
    onClear: noop,
    onCompose: noop,
    onCopied: noop,
  };
  return <Jobs {...props} />;
}

function notes(): string[] {
  return Array.from(document.querySelectorAll(".armada-job-row__changed"), (note) => note.textContent ?? "");
}

function rowOf(id: string): HTMLElement {
  const row = document.querySelector<HTMLElement>(`[data-job-id="${id}"]`);
  if (row === null) throw new Error(`no row for ${id}`);
  return row;
}

test("the first reading marks no row", async () => {
  mount(board([job("running", { id: "a", title: "First" }), job("escalated", { id: "b", title: "Second" })]));
  await expect.poll(() => document.querySelectorAll("[data-job-id]").length).toBe(2);
  expect(notes()).toEqual([]);
});

test("a status change marks its row, in the badge's own word, and no other row", async () => {
  const a = job("running", { id: "a", title: "First" });
  const b = job("running", { id: "b", title: "Second", created_at: "2026-09-10T15:00:00Z" });
  mount(board([a, b]));
  await expect.poll(() => document.querySelectorAll("[data-job-id]").length).toBe(2);

  rerender(board([{ ...a, status: "awaiting_review" }, b]));
  await expect.poll(() => rowOf("a").querySelector(".armada-job-row__changed")?.textContent).toMatch(AGO);
  const note = rowOf("a").querySelector(".armada-job-row__changed")?.textContent ?? "";
  const badge = rowOf("a").querySelector(".armada-badge")?.textContent ?? "";
  expect(note.toLowerCase().startsWith(badge.toLowerCase())).toBe(true);
  expect(rowOf("b").querySelector(".armada-job-row__changed")).toBeNull();
  // It starts at the recent strength: the row reads a stronger mix than its resting neighbour.
  expect(rowOf("a").style.getPropertyValue("--armada-row-recent")).not.toBe("");
});

test("a Job arriving on the Board is a new row, not a changed one", async () => {
  const a = job("running", { id: "a", title: "First" });
  mount(board([a]));
  await expect.poll(() => document.querySelectorAll("[data-job-id]").length).toBe(1);

  rerender(board([a, job("awaiting_review", { id: "c", title: "Arrived" })]));
  await expect.poll(() => document.querySelectorAll("[data-job-id]").length).toBe(2);
  expect(notes()).toEqual([]);
});
