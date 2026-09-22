// The milestone's claim: what a person sees at each moment of the arc, through
// `App` and nothing else.
//
// **Written before the screens, and mostly still owed.** Each `it.todo` below
// is one moment of #1533's roster, in the words of what somebody looking at
// the window should be able to read. The issue that builds a board turns its
// own todos into passing tests — #1535 the Plan, #1536 Implement, #1537 the
// Record, #1538 Pulse, #1539 the canvas, #1540 Dispatch, #1541 classifying,
// #1542 Land, #1543 landing in order, #1544 the wave.
//
// **A todo names what a person sees, never a component.** A claim written
// against `RunningPanel` would have to be rewritten by whoever renames it; one
// written against what is on screen survives the rename and fails the day the
// sentence stops being true.

import { expect, test, describe, it } from "vitest";
import { page } from "vitest/browser";

import { SCENARIOS, scenarioNamed } from "./scenario";
import { mount, openBoard, rows, unmountAfterEach } from "./testing";

unmountAfterEach();

/**
 * A moment, with one of the Job's five destinations open. **The tab is
 * pressed the way a person presses it** — the strip is the whole of navigation
 * inside a Job, so a test that reached a destination any other way would pass
 * over a strip that had stopped working.
 */
async function at(moment: string, tab: string) {
  mount(moment);
  await expect.element(page.getByRole("tab", { name: new RegExp(`^${tab}`) })).toBeVisible();
  await page.getByRole("tab", { name: new RegExp(`^${tab}`) }).click();
  await expect.element(page.getByRole("tabpanel", { name: tab })).toBeVisible();
}

/**
 * One group's card, by the heading it carries. **Scoped to the groups list and
 * matched case-sensitively**: the warning above the cards names the same
 * groups in lower case, and a substring match reached it first.
 */
const groupCard = (ordinal: number) =>
  page
    .getByRole("list", { name: "Groups, in the order they run" })
    .getByRole("listitem")
    .filter({ hasText: new RegExp(`Group ${ordinal}`) })
    .first();

/** One task's row, by the name the board gives it. */
const taskRow = (id: string) => page.getByRole("listitem", { name: new RegExp(`^${id} `) });

/** Every scenario the arc put in the picker, by name. */
const ARC = SCENARIOS.filter((one) => one.name.startsWith("arc/")).map((one) => one.name);

// What can be claimed today: each moment loads, and the window draws rather
// than blanking. The boards are what the todos below wait on.
describe("every arc moment loads", () => {
  test.for(ARC)("%s draws a window", async (name) => {
    const { scenario } = mount(name);
    expect(scenario.draft).toBeDefined();
    await expect.element(page.getByRole("navigation").first()).toBeVisible();
  });

  test("the picker's moments are in the order the work happens", () => {
    expect(ARC[0]).toBe("arc/dispatch-typing");
    expect(ARC[ARC.length - 1]).toBe("arc/landed");
  });

  test.for(["members/stacked", "members/merged", "epic/wave", "kinds"])(
    "%s draws a window",
    async (name) => {
      expect(scenarioNamed(name), `no scenario named ${name}`).toBeDefined();
      mount(name);
      await expect.element(page.getByRole("navigation").first()).toBeVisible();
    },
  );

  test("the kinds Board opens every Job it holds", async () => {
    const { scenario } = mount("kinds");
    await openBoard();
    const done = page.getByRole("button", { name: /^Done/ });
    if (done.query()?.getAttribute("aria-expanded") === "false") await done.click();
    await expect
      .poll(() => rows().map((row) => row.dataset.jobId).sort())
      .toEqual(scenario.state.jobs.map((job) => job.id).sort());
  });
});

describe("dispatch", () => {
  test(
    "arc/dispatch-typing: the prompt a person is typing stands beside the two Jobs already " +
      "writing where this work would, each named with the path they share — and nothing is " +
      "greyed out, because an overlap is a fact and not a refusal",
    async () => {
      mount("arc/dispatch-typing");

      const field = page.getByRole("textbox", { name: "Request" });
      await expect.element(field).toHaveValue(expect.stringContaining("Drones 1 of 2"));

      const beside = page.getByText("What else is running").first();
      await expect.element(beside).toBeVisible();
      await expect
        .element(page.getByText("Fold the capacity read into one query"))
        .toBeVisible();
      // The chip's title is the whole path, which is what a person reads on
      // hover and what survives the directory's own clip.
      await expect.element(page.getByTitle("crates/api/src/")).toBeVisible();
      await expect.element(page.getByText("Give the rail its own scroll")).toBeVisible();
      await expect.element(page.getByTitle("packages/screens/src/overview.ts")).toBeVisible();

      // The claim the panel exists for: it says what is running and stops
      // nothing. A greyed Dispatch would make the reading a refusal.
      await expect
        .element(page.getByRole("button", { name: "Dispatch", disabled: true }))
        .not.toBeInTheDocument();
    },
  );

  test(
    "arc/dispatch-typing: the form says how many Drones this Job may run at once, and what " +
      "the machine allows across every Job, as two different numbers",
    async () => {
      mount("arc/dispatch-typing");
      // The block's own head, which says how many are set — the rail carries a
      // Settings of its own.
      await page.getByRole("button", { name: /Settings \d set/ }).click();

      await expect.element(page.getByRole("spinbutton", { name: "Drones at once" })).toHaveValue(2);
      await expect.element(page.getByText("This machine runs 4 at once")).toBeVisible();
    },
  );

  test(
    "arc/dispatch-typing: where the work starts and where it lands are two fields, and both " +
      "read main without either one being called the other's default",
    async () => {
      mount("arc/dispatch-typing");

      await expect.element(page.getByRole("textbox", { name: "From" })).toHaveValue("main");
      await expect.element(page.getByRole("textbox", { name: "Lands in" })).toHaveValue("main");
      // Neither field says what the other does. A parenthetical default is how
      // the pair collapses back into the one field it used to be.
      await expect.element(page.getByText("default", { exact: false })).not.toBeInTheDocument();
    },
  );
  it.todo(
    "arc/dispatch-sketch: the picture a person drew is on screen beside the prompt, with what " +
      "they said about it and the Studio node it was made from",
  );
});

describe("classifying", () => {
  it.todo(
    "arc/proposing-reading: the form says the proposer is thinking, how long it has been out " +
      "and how long it may take — and offers to stop it",
  );
  it.todo(
    "arc/proposing-review: the workflow the proposer chose is named with its four steps, and " +
      "every one of them has a gate row a person can still change",
  );
  it.todo(
    "arc/proposing-review: the handoff step reads that the repository decides, which is not " +
      "one of the three tick boxes beside it",
  );
  it.todo(
    "arc/proposing-review: one permanent line says Fleet checks the work stayed inside the " +
      "plan, and no tick turns it off",
  );
  it.todo(
    "arc/proposing-review: each of the three tiers names the model it resolves to, and the " +
      "planner's tier is what picks it",
  );
  it.todo(
    "arc/approved-frozen: the Job says when it was approved, and that what it is held to was " +
      "frozen at that moment",
  );
  it.todo(
    "arc/approved-frozen: the first criterion says the issue it came from has been edited " +
      "since, and the Job still shows the words it froze",
  );
  it.todo(
    "arc/approved-frozen: the handoff gate reads that this Job overrode the repository's rule " +
      "for itself",
  );
});

describe("the plan", () => {
  test("arc/planned: the Plan tab draws four groups in the order they run, with eight tasks under them and no task in two groups", async () => {
    await at("arc/planned", "Plan");
    const groups = page.getByRole("list", { name: "Groups, in the order they run" });
    await expect.element(groups).toBeVisible();
    const headings = [...groups.element().querySelectorAll("h3")].map((one) => one.textContent);
    expect(headings).toEqual(["Group 1", "Group 2", "Group 3", "Group 4"]);
    const ids = [...groups.element().querySelectorAll("[aria-label$='tasks'] > li")].map(
      (one) => one.getAttribute("aria-label")?.split(" ")[0],
    );
    expect(ids).toEqual(["T1", "T2", "T3", "T4", "T5", "T6", "T7", "T8"]);
  });

  test("arc/planned: each task names the files it will touch, the tier the planner gave it, and the model that tier resolved to", async () => {
    await at("arc/planned", "Plan");
    await expect.element(taskRow("T1")).toHaveTextContent("difficult · opus");
    await expect.element(taskRow("T3")).toHaveTextContent("easy · haiku");
    await expect
      .element(page.getByRole("list", { name: "Group 1 scope" }))
      .toHaveTextContent("running.rs");
  });

  test("arc/planned: each group says which Checks run at its end — four where it writes Rust, seven where it writes Bridge", async () => {
    await at("arc/planned", "Plan");
    await expect.element(groupCard(1)).toHaveTextContent("4 checks will run at this boundary");
    await expect.element(groupCard(1)).toHaveTextContent("acceptance");
    await expect.element(groupCard(2)).toHaveTextContent("7 checks will run at this boundary");
    await expect.element(groupCard(2)).toHaveTextContent("components_test");
  });

  test("arc/planned: the case that covers a file two groups touch is drawn at the last of them, and the case with no spec reads as not covered rather than as passing", async () => {
    await at("arc/planned", "Plan");
    await expect.element(groupCard(4)).toHaveTextContent("overview.test.ts");
    await expect.element(groupCard(2)).not.toHaveTextContent("overview.test.ts");
    await expect.element(groupCard(2)).toHaveTextContent("Board.test.tsx");
    await expect.element(groupCard(2)).toHaveTextContent("not covered");
  });

  test("arc/planned: a task's inspector says what its Drone will be told, what it may run beside and the tests it owes", async () => {
    await at("arc/planned", "Plan");
    await taskRow("T5").getByRole("button").first().click();
    const sheet = page.getByRole("dialog").first();
    await expect.element(sheet).toHaveTextContent("What its Drone will be told");
    await expect.element(sheet).toHaveTextContent("difficult · opus · its own agent");
    await expect.element(sheet).toHaveTextContent("T6");
    await expect.element(sheet).toHaveTextContent("Running.test.tsx");
    await expect.element(sheet).toHaveTextContent("What the planner holds it to");
  });

  test("arc/planned: a file two groups claim is named as a warning, with both groups and both tasks", async () => {
    await at("arc/planned", "Plan");
    const warning = page.getByRole("region", { name: "Two groups claim the same file" });
    await expect.element(warning).toHaveTextContent("running-rows.tsx");
    await expect.element(warning).toHaveTextContent("group 3 (T6) and group 4 (T7)");
  });

  test("arc/group-failed: the Plan tab draws group three's failure with the one Check that failed named, and the six that passed beside it", async () => {
    await at("arc/group-failed", "Plan");
    await expect.element(groupCard(3)).toHaveTextContent("failed at its checks");
    await expect.element(groupCard(3)).toHaveTextContent("second run");
    await expect.element(groupCard(3)).toHaveTextContent("screens_test");
    await expect.element(groupCard(3)).toHaveTextContent("typecheck");
    await expect.element(taskRow("T6")).toHaveTextContent("opened the Board rather than the Job");
  });

  test("arc/done-touched: T6 still reads done on the Plan tab and carries a flag naming T7", async () => {
    await at("arc/done-touched", "Plan");
    await expect.element(taskRow("T6")).toHaveTextContent("touched later · T7");
    await expect.element(taskRow("T6").getByText("Done")).toBeInTheDocument();
    await expect.element(groupCard(3)).toHaveTextContent("passed");
    await expect.element(taskRow("T7")).toHaveTextContent("6 turns");
  });

  test("arc/done-touched: a finished task shows what its own agent cost and a working one does not", async () => {
    await at("arc/done-touched", "Plan");
    await expect.element(taskRow("T1")).toHaveTextContent("34 turns · ~$2.40");
    await expect.element(taskRow("T7")).not.toHaveTextContent("$");
  });

  it.todo(
    "arc/plan-revision-refused: the Judge's refusal names the one task that was revised, and " +
      "the other seven are untouched beside it",
  );
  it.todo(
    "arc/plan-revision-refused: the case that fell out of the revised scope reads dropped, " +
      "with the revision that dropped it",
  );
});

describe("implement", () => {
  it.todo(
    "arc/executing-sequential: groups one and two read passed with the commit each left, " +
      "group three is working, and group four has not started",
  );
  it.todo(
    "arc/executing-sequential: the working task shows its turns and no cost, because its " +
      "agent has not stopped",
  );
  it.todo(
    "arc/executing-sequential: every finished task shows what it cost, including the ones in " +
      "a group that has already passed",
  );
  it.todo(
    "arc/executing-concurrent: T5 and T6 are drawn as having run at the same time, each with " +
      "its own agent, and group three reads joining",
  );
  it.todo(
    "arc/executing-concurrent: both tasks show a cost the moment their own agent stopped, " +
      "before their group has been checked",
  );
  it.todo(
    "arc/group-failed: group three reads failed with the one Check that failed named, and " +
      "says this is its second run",
  );
  it.todo(
    "arc/group-failed: the six Checks that passed are drawn beside the one that did not, " +
      "rather than the group reading red with nothing said",
  );
  it.todo(
    "arc/done-touched: T6 still reads done and carries a flag saying a later task edited the " +
      "file it had finished, and T7 is named as the task that did",
  );
});

describe("the Record", () => {
  test(
    "arc/planned: the Record's who column says Judge on the row where the plan was judged, " +
      "and Fleet on the row where the plan was recorded",
    async () => {
      await record("arc/planned");
      const rows = ledger();

      expect(whoSaid(rows, /Pressing the stat lists the Drone's Job and step/)).toBe("Judge");
      expect(whoSaid(rows, /^the plan was recorded$/)).toBe("Fleet");
    },
  );

  test(
    "arc/executing-sequential: a Check's run is its own row, with Check in the who column — " +
      "not Fleet, and not the Drone that produced the work",
    async () => {
      await record("arc/executing-sequential");
      const rows = ledger();

      expect(whoSaid(rows, /^typecheck$/)).toBe("Check");
      expect(whoSaid(rows, /^screens_test$/)).toBe("Check");
      // The same Job's Drone and Fleet are both on screen, so Check is a
      // distinction the table is drawing rather than the only word it has.
      expect(rows.map((row) => row["Who ran it"])).toContain("Drone");
      expect(rows.map((row) => row["Who ran it"])).toContain("Fleet");
    },
  );

  // **The group is named where it holds more than one task.** Today's wire has
  // no groups, so `taskGroupsOf` derives one per task — and `Implement · group
  // 1 · T1` would then put the same fact on the row twice. The coordinate
  // still carries the group; `draft/ledger.test.ts` is where that is pinned.
  test(
    "arc/executing-sequential: each row says where in the Job it happened, down to the task " +
      "where it has one",
    async () => {
      await record("arc/executing-sequential");
      const rows = ledger();

      expect(rows.every((row) => row["Where"] !== "")).toBe(true);
      expect(rows.map((row) => row["Where"])).toContain("Implement · T1");
      expect(rows.map((row) => row["Where"])).toContain("Plan the change");
    },
  );

  // **Reworded from "the approval row".** The instant a Job was approved is a
  // status move, and status moves live in `GET /jobs/:job_id/events`, which no
  // arc fixture answers — so the row carrying this claim is the Job's creation.
  // The claim itself is unchanged: a fact about the Job names no step.
  test(
    "arc/approved-frozen: the row for the Job's own machine moving names no step at all, " +
      "because that is a fact about the Job",
    async () => {
      await record("arc/approved-frozen");
      const rows = ledger();

      expect(rows.map((row) => row["Where"])).toContain("The Job itself");
      expect(whoSaid(rows, /^this Job was created$/)).toBe("You");
    },
  );
});

/** App on an arc moment, with the Record open. */
async function record(name: string): Promise<void> {
  mount(name);
  await page.getByRole("tab", { name: /^Record/ }).click();
  await expect.poll(() => ledger().length).toBeGreaterThan(0);
}

/**
 * The Record's rows, each keyed by the column its cells sit under.
 *
 * Read through the table's own header rather than by position, so a column
 * added or reordered moves the keys with it instead of silently shifting every
 * assertion one cell along.
 */
function ledger(): Record<string, string>[] {
  const table = document.querySelector("table");
  if (table === null) return [];
  const columns = [...table.querySelectorAll("thead th")].map((one) => one.textContent?.trim() ?? "");
  return [...table.querySelectorAll("tbody tr")].map((row) =>
    Object.fromEntries(
      [...row.querySelectorAll("td")].map((cell, at) => [
        columns[at] ?? String(at),
        cell.textContent?.trim() ?? "",
      ]),
    ),
  );
}

/** Who ran the one row whose What matches. */
function whoSaid(rows: Record<string, string>[], what: RegExp): string | undefined {
  return rows.find((row) => what.test(row["What"] ?? ""))?.["Who ran it"];
}

describe("Pulse", () => {
  test("arc/executing-sequential: Pulse names the one process the running Drone holds, the worktree it belongs to, and the instant every figure was read at", async () => {
    mount("arc/executing-sequential");
    await onPulse();

    const processes = page.getByRole("region", { name: "Processes" });
    await expect.element(processes.getByText("52118")).toBeVisible();
    // The branch is on the process row and on the worktree row, which is the
    // whole of "the worktree it belongs to": one occurrence is a table that
    // lists what is running and does not say where.
    expect(processes.getByText(ARC_BRANCH).all()).toHaveLength(1);
    await expect
      .element(page.getByRole("region", { name: "Worktrees" }).getByText(ARC_BRANCH))
      .toBeVisible();
    await expect.element(page.getByText(/^Read .* ago\./)).toBeVisible();
  });

  test("arc/executing-concurrent: with no agent running, Pulse says so rather than drawing an empty table", async () => {
    mount("arc/executing-concurrent");
    await onPulse();

    await expect
      .element(page.getByText(/Fleet holds no process for this job/))
      .toBeVisible();
    expect(page.getByRole("columnheader", { name: /Process/ }).query()).toBeNull();
  });

  // The parent of three landings holds a plan and no checkout of its own, so
  // every one of Pulse's lists is empty here — which is the state each of them
  // has its own sentence for. It is also the one moment that proves none of
  // them draws a bare empty table. Nothing on screen names a shape (#1530).
  test("members/stacked: the parent says what it holds rather than drawing three empty lists", async () => {
    mount("members/stacked");
    await onPulse();

    await expect.element(page.getByText(/the process table would not read/)).toBeVisible();
    await expect.element(page.getByText("No worktree on disk.")).toBeVisible();
    await expect.element(page.getByText(/Nothing has been written to this job's logs/)).toBeVisible();
    expect(document.body.textContent).not.toMatch(/convoy|train/i);
  });
});

/** The branch the arc's one Job works on — `arc-base.ts`'s own spelling. */
const ARC_BRANCH = "armada/3-show-what-s-running-in-the-drones-stat";

/** Open Pulse on the Job the moment opens, the way a person reaches it. */
async function onPulse(): Promise<void> {
  await page.getByRole("tab", { name: "Pulse" }).click();
  await expect.element(page.getByRole("tabpanel", { name: "Pulse" })).toBeVisible();
}

describe("landing", () => {
  it.todo(
    "arc/landed: Land shows the whole test set run again before the pull request was " +
      "offered, with the one case that has no spec named as not covered",
  );
  it.todo(
    "arc/landed: the run a person made themselves is drawn beside Fleet's, and says which of " +
      "them ran it",
  );
  it.todo("arc/landed: the pull request is on screen as an address a press opens");
  it.todo(
    "members/stacked: three pull requests are drawn in the order they land — one merged, one " +
      "waiting on you, one stacked on it — and nothing on screen names a shape",
  );
  it.todo(
    "members/merged: the third member reads as merged into the one before it and parked as a " +
      "draft, so nothing is asked of a reviewer yet",
  );
  it.todo(
    "members/stacked: the parent says it is done when every member has landed, which is not " +
      "the same as its own pull request merging",
  );
});

describe("the wave", () => {
  it.todo(
    "epic/wave: five Jobs are drawn under one plan with what each has reached, and the two " +
      "that merged are told apart from the three still out",
  );
  it.todo("epic/wave: pressing one of the wave's Jobs opens that Job");
});

describe("one Job per workflow kind", () => {
  it.todo(
    "kinds: each of the eight Jobs draws the steps its own workflow file declares — three " +
      "for the bug Job, two for the revert Job, four for the feature Job",
  );
  it.todo("kinds: no Job on the Board runs a workflow called verify-and-ship");
  it.todo(
    "kind/code-review: the run ends at a step that delivers a review, and no step of it " +
      "offers a diff to land",
  );
});
