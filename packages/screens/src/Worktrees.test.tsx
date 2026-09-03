// Choosing which held worktrees to give back, and what the screen says before
// it does.
//
// # Why these are browser tests and not stories
//
// `packages/components` sits below this package, so no story there can mount
// this surface — a story proves what one held row draws, and this proves what
// the surface does with a set of them. What is asserted here is behaviour a
// rendering cannot show: that the act is per item, that a job still running is
// drawn and never offered, that the confirmation reads out the files it is
// about to destroy, and that a refusal on one job does not swallow the rest.
//
// The arithmetic is next door in `held.test.ts`, where a hundred cases cost
// what one costs.

import { afterEach, expect, test } from "vitest";
import { page, userEvent } from "vitest/browser";
import type { HeldWorktrees, Outcome, WorktreeHeld } from "@armada/protocol";

import { mount, unmount } from "./mounted";
import { Worktrees } from "./Worktrees";

afterEach(unmount);

/** Stable, because the surface depends on it in an effect. */
const WANT = (): void => {};

function held(over: Partial<WorktreeHeld> = {}): WorktreeHeld {
  return {
    job_id: "01JOB0001",
    job_title: "Port the settings selectors",
    status: "completed_success",
    path: "/Users/user/armada/.armada/worktrees/01JOB0001",
    branch: "armada/01JOB0001",
    held: [],
    ...over,
  };
}

const UNMERGED = { why: "unmerged", base: "main", commits: 3, tip: "9f1c2ab84d5e" } as const;

/** Mount the surface over one answer, and hand back what was reclaimed. */
function opened(
  worktrees: WorktreeHeld[],
  answer: (jobId: string) => Outcome = () => ({ ok: true }),
): { sent: string[] } {
  const sent: string[] = [];
  const read: HeldWorktrees = { state: "read", held: { worktrees } };
  mount(
    <Worktrees
      onWant={WANT}
      held={read}
      onReclaim={(jobId) => {
        sent.push(jobId);
        return Promise.resolve(answer(jobId));
      }}
      onCopied={() => {}}
    />,
  );
  return { sent };
}

function reclaim() {
  return page.getByRole("button", { name: /^Reclaim/ });
}

/**
 * **The act is per item and there is no select-all.** `armada clean
 * --everything` is the one bulk act in armada and it is the one nobody should
 * reach for from a screen; a control here that took the whole list would be
 * that act with a friendlier name.
 */
test("nothing is chosen until somebody chooses it, one row at a time", async () => {
  const { sent } = opened([
    held({ job_id: "a", job_title: "First", held: [UNMERGED] }),
    held({ job_id: "b", job_title: "Second", held: [UNMERGED] }),
  ]);

  await expect.element(reclaim()).toBeDisabled();
  // Two rows, two checkboxes, and no third control that takes both.
  expect(page.getByRole("checkbox").elements()).toHaveLength(2);

  await userEvent.click(page.getByRole("checkbox", { name: "First" }));
  await expect.element(reclaim()).toHaveTextContent("Reclaim 1 worktree");

  await userEvent.click(reclaim());
  await userEvent.click(page.getByRole("dialog").getByRole("button", { name: "Reclaim" }));
  expect(sent, "only the row that was chosen").toEqual(["a"]);
});

/**
 * **A job that has not ended is drawn and never offered.** Fleet refuses the
 * act on a status that is not terminal, so a checkbox would be a control whose
 * only outcome is a refusal — and the row still has to be on the page, or a
 * worktree missing from the list above reads as disk already returned.
 */
test("a job still running is on the page with no control on it", async () => {
  opened([
    held({ job_id: "a", job_title: "Finished", held: [UNMERGED] }),
    held({
      job_id: "b",
      job_title: "Still going",
      status: "running",
      held: [{ why: "not_terminal", status: "running" }],
    }),
  ]);

  await expect.element(page.getByText("Still going", { exact: true })).toBeInTheDocument();
  expect(page.getByRole("checkbox").elements(), "one checkbox, not two").toHaveLength(1);
  await expect
    .element(page.getByRole("checkbox", { name: "Finished" }))
    .toBeInTheDocument();
});

/**
 * **The confirmation says what is lost, and never how much disk comes back.**
 * Uncommitted files are the only thing the act ends — no branch carries them —
 * so they are read out by name, and the branch that survives is said to survive
 * rather than left to be assumed either way.
 */
test("the confirmation names the files it destroys and the branch it keeps", async () => {
  opened([
    held({
      job_id: "a",
      job_title: "Trial the judge prompt",
      held: [UNMERGED, { why: "uncommitted", files: ["crates/config/src/judge.rs"] }],
    }),
  ]);

  await userEvent.click(page.getByRole("checkbox", { name: "Trial the judge prompt" }));
  await userEvent.click(reclaim());

  const dialog = page.getByRole("dialog");
  await expect.element(dialog).toHaveTextContent("One file is destroyed");
  await expect.element(dialog).toHaveTextContent("crates/config/src/judge.rs");
  await expect.element(dialog).toHaveTextContent("These branches are kept");
  await expect.element(dialog).toHaveTextContent("armada/01JOB0001");
});

/**
 * The ordinary case, and it is said rather than left as an absence. With no
 * force on this seam, most reclaims end nothing at all — a confirmation that
 * listed nothing would read as one that failed to say what it costs.
 */
test("a reclaim that ends nothing says so", async () => {
  opened([held({ job_id: "a", job_title: "Finished", held: [UNMERGED] })]);

  await userEvent.click(page.getByRole("checkbox", { name: "Finished" }));
  await userEvent.click(reclaim());

  await expect.element(page.getByRole("dialog")).toHaveTextContent("Nothing is lost.");
});

/**
 * **One refusal does not swallow the rest.** There is no bulk route on the
 * wire, so this is one call per id and some can refuse while others land — and
 * the one that refused has to be named, or a person is left comparing the list
 * against what they remember choosing.
 */
test("a refusal on one job is named and the others still go", async () => {
  const { sent } = opened(
    [
      held({ job_id: "a", job_title: "First", held: [UNMERGED] }),
      held({ job_id: "b", job_title: "Second", held: [UNMERGED] }),
    ],
    (jobId) =>
      jobId === "a" ? { ok: false, why: "not_connected" } : { ok: true },
  );

  await userEvent.click(page.getByRole("checkbox", { name: "First" }));
  await userEvent.click(page.getByRole("checkbox", { name: "Second" }));
  await userEvent.click(reclaim());
  await userEvent.click(page.getByRole("dialog").getByRole("button", { name: "Reclaim" }));

  expect(sent, "the second was sent after the first refused").toEqual(["a", "b"]);
  await expect
    .element(page.getByText("One worktree was not given back"))
    .toBeInTheDocument();
});

/**
 * **Fleet's own half is on this page too.** A worktree the sweep is about to
 * take is drawn rather than filtered out: a person who came looking for one and
 * does not find it cannot tell "already given back" from "held and not said".
 */
test("what fleet takes on its own is drawn, and offers nothing", async () => {
  opened([held({ job_id: "a", job_title: "Already safe" })]);

  await expect.element(page.getByText("Nothing is waiting on you")).toBeInTheDocument();
  await expect.element(page.getByText("Already safe")).toBeInTheDocument();
  expect(page.getByRole("checkbox").elements()).toHaveLength(0);
});

/**
 * A failed read is a failure of the read and never an empty list. An empty list
 * here would claim fleet is holding nothing, which is the one answer on this
 * page nobody should be given by accident.
 */
test("a read that failed says so rather than drawing an empty page", async () => {
  mount(
    <Worktrees
      onWant={WANT}
      held={{ state: "failed", outcome: { ok: false, why: "not_connected" } }}
      onReclaim={() => Promise.resolve({ ok: true })}
      onCopied={() => {}}
    />,
  );

  await expect
    .element(page.getByText("What fleet is holding could not be read"))
    .toBeInTheDocument();
  expect(page.getByRole("checkbox").elements()).toHaveLength(0);
});
