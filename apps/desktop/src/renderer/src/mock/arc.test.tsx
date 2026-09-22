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
  it.todo(
    "arc/dispatch-typing: the prompt a person is typing stands beside the two Jobs already " +
      "writing where this work would, each named with the path they share — and nothing is " +
      "greyed out, because an overlap is a fact and not a refusal",
  );
  it.todo(
    "arc/dispatch-typing: the form says how many Drones this Job may run at once, and what " +
      "the machine allows across every Job, as two different numbers",
  );
  it.todo(
    "arc/dispatch-typing: where the work starts and where it lands are two fields, and both " +
      "read main without either one being called the other's default",
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

  test("arc/plan-revision-refused: the Judge's refusal names the one task that was revised, and the other seven are untouched beside it", async () => {
    await at("arc/plan-revision-refused", "Plan");
    const asked = page.getByRole("region", {
      name: "What you asked the plan's Drone to change",
    });
    await expect.element(asked).toHaveTextContent("out of T5");
    await expect.element(asked).toHaveTextContent("Refused");
    await expect
      .element(asked)
      .toHaveTextContent("the plan names every file the panel's rows are drawn from");
    await expect.element(asked).toHaveTextContent("no other task claims it");
    await expect
      .element(asked)
      .toHaveTextContent("T5 is the one task the ask touched. The other 7 stand");
    // The plan itself is the one the Drone recorded: eight tasks, four groups.
    const ids = [
      ...page
        .getByRole("list", { name: "Groups, in the order they run" })
        .element()
        .querySelectorAll("[aria-label$='tasks'] > li"),
    ].map((one) => one.getAttribute("aria-label")?.split(" ")[0]);
    expect(ids).toEqual(["T1", "T2", "T3", "T4", "T5", "T6", "T7", "T8"]);
  });

  test("arc/plan-revision-refused: the case that fell out of the revised scope reads dropped, with the revision that dropped it", async () => {
    await at("arc/plan-revision-refused", "Plan");
    await expect.element(groupCard(4)).toHaveTextContent("Running.test.tsx");
    await expect.element(groupCard(4)).toHaveTextContent("dropped by a scope revision");
    await expect
      .element(page.getByRole("region", { name: "What you asked the plan's Drone to change" }))
      .toHaveTextContent("It drops packages/screens/src/Running.test.tsx.");
  });

  test("arc/plan-revision-refused: a group offers move up, move down and remove while the plan waits, and the first group cannot move up", async () => {
    await at("arc/plan-revision-refused", "Plan");
    const asks = page.getByRole("group", { name: "Ask about group 1" });
    await expect.element(asks.getByRole("button", { name: "Move up" })).toBeDisabled();
    await expect.element(asks.getByRole("button", { name: "Move down" })).toBeEnabled();
    await expect.element(asks.getByRole("button", { name: "Remove" })).toBeEnabled();
    await expect
      .element(page.getByRole("tabpanel", { name: "Plan" }))
      .toHaveTextContent("The plan is the Drone's record");
  });

  test("arc/plan-revision-refused: moving a group past one that claims the same file warns before the ask goes out", async () => {
    await at("arc/plan-revision-refused", "Plan");
    await page
      .getByRole("group", { name: "Ask about group 3" })
      .getByRole("button", { name: "Move down" })
      .click();
    const dialog = page.getByRole("dialog").first();
    await expect
      .element(dialog)
      .toHaveTextContent("Ask the Drone to run group 3 after group 4?");
    await expect
      .element(dialog)
      .toHaveTextContent("Group 3 and group 4 both claim packages/screens/src/running-rows.tsx");
    await expect.element(dialog).toHaveTextContent("Group 3 writes it first as the plan stands");
    await expect.element(dialog).toHaveTextContent("it may refuse");
  });

  test("arc/plan-revision-refused: a reorder the scopes do not disagree with carries no warning", async () => {
    await at("arc/plan-revision-refused", "Plan");
    await page
      .getByRole("group", { name: "Ask about group 2" })
      .getByRole("button", { name: "Move up" })
      .click();
    const dialog = page.getByRole("dialog").first();
    await expect
      .element(dialog)
      .toHaveTextContent("Ask the Drone to run group 2 before group 1?");
    await expect.element(dialog).not.toHaveTextContent("both claim");
  });

  test("arc/plan-revision-refused: a task's inspector asks for a rewrite, and the control is off until something is typed", async () => {
    await at("arc/plan-revision-refused", "Plan");
    await taskRow("T6").getByRole("button").first().click();
    const sheet = page.getByRole("dialog").first();
    await expect.element(sheet).toHaveTextContent("Rewrite this task");
    const send = sheet.getByRole("button", { name: "Ask the Drone" });
    await expect.element(send).toBeDisabled();
    await sheet.getByRole("textbox").first().fill("Split the rows out of this one");
    await expect.element(send).toBeEnabled();
  });

  test("arc/planned: a plan already past its gate offers no changes at all", async () => {
    await at("arc/planned", "Plan");
    await expect.element(page.getByRole("tabpanel", { name: "Plan" })).toBeVisible();
    expect(page.getByRole("group", { name: "Ask about group 1" }).elements()).toHaveLength(0);
    await expect
      .element(page.getByRole("tabpanel", { name: "Plan" }))
      .not.toHaveTextContent("The plan is the Drone's record");
  });
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
  it.todo(
    "arc/planned: the Record's who column says Judge on the row where the plan was judged, " +
      "and Fleet on the row where the plan was recorded",
  );
  it.todo(
    "arc/executing-sequential: a Check's run is its own row, with Check in the who column — " +
      "not Fleet, and not the Drone that produced the work",
  );
  it.todo(
    "arc/executing-sequential: each row says where in the Job it happened, down to the group " +
      "and the task where it has one",
  );
  it.todo(
    "arc/approved-frozen: the approval row names no step at all, because the Job's own " +
      "machine moving is a fact about the Job",
  );
});

describe("Pulse", () => {
  it.todo(
    "arc/executing-sequential: Pulse names the one process the running Drone holds, the " +
      "worktree it belongs to, and the instant every figure was read at",
  );
  it.todo(
    "arc/executing-concurrent: with no agent running, Pulse says so rather than drawing an " +
      "empty table",
  );
});

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
