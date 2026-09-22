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
  test(
    "arc/executing-sequential: groups one and two read passed with the commit each left, " +
      "group three is working, and group four has not started",
    async () => {
      await at("arc/executing-sequential", "Workflow");

      await expect.element(runGroup(1)).toHaveTextContent("passed");
      await expect.element(runGroup(1)).toHaveTextContent("4c1b9d2");
      await expect.element(runGroup(2)).toHaveTextContent("passed");
      await expect.element(runGroup(2)).toHaveTextContent("7a2f0c5");
      await expect.element(runGroup(3)).toHaveTextContent("working");
      await expect.element(runGroup(4)).toHaveTextContent("not started");
      // One group at a time is the rule, and this line is its only evidence.
      await expect.element(page.getByText(/No task of the next group starts/)).toBeVisible();
    },
  );

  test(
    "arc/executing-sequential: the working task shows its turns and no cost, because its " +
      "agent has not stopped",
    async () => {
      await at("arc/executing-sequential", "Workflow");

      // Group three is the one moving, so it opens itself.
      await expect.element(runTask("T5")).toHaveTextContent("14 turns");
      await expect.element(runTask("T5")).not.toHaveTextContent("$");
    },
  );

  test(
    "arc/executing-sequential: every finished task shows what it cost, including the ones in " +
      "a group that has already passed",
    async () => {
      await at("arc/executing-sequential", "Workflow");
      await openGroup(1);
      await openGroup(2);

      await expect.element(runTask("T1")).toHaveTextContent("34 turns · ~$2.40");
      await expect.element(runTask("T2")).toHaveTextContent("12 turns · ~$0.64");
      await expect.element(runTask("T3")).toHaveTextContent("8 turns · ~$0.26");
      await expect.element(runTask("T4")).toHaveTextContent("9 turns · ~$0.31");
    },
  );

  test(
    "arc/executing-concurrent: T5 and T6 are drawn as having run at the same time, each with " +
      "its own agent, and group three reads joining",
    async () => {
      await at("arc/executing-concurrent", "Workflow");

      await expect.element(runGroup(3)).toHaveTextContent("joining its work");
      await expect.element(runGroup(3)).toHaveTextContent("2 tasks, at the same time");
      // What bounds the fan out, settled at the gate — `#1550`.
      await expect.element(runGroup(3)).toHaveTextContent("this Job runs 2 Drones at once");
      await expect.element(runTask("T5")).toHaveTextContent("its own agent");
      await expect.element(runTask("T5")).toHaveTextContent("runs beside T6");
      await expect.element(runTask("T6")).toHaveTextContent("its own agent");
      await expect.element(runTask("T6")).toHaveTextContent("runs beside T5");
    },
  );

  test(
    "arc/executing-concurrent: both tasks show a cost the moment their own agent stopped, " +
      "before their group has been checked",
    async () => {
      await at("arc/executing-concurrent", "Workflow");

      await expect.element(runTask("T5")).toHaveTextContent("27 turns · ~$1.90");
      await expect.element(runTask("T6")).toHaveTextContent("15 turns · ~$0.72");
      // The boundary has not run, and the two costs are on screen anyway.
      const boundary = runGroup(3).getByRole("region", { name: "Checks at this boundary" });
      await expect.element(boundary).toHaveTextContent("7 checks will run at this boundary");
      await expect.element(boundary).toHaveTextContent("not run");
    },
  );

  test(
    "arc/group-failed: group three reads failed with the one Check that failed named, and " +
      "says this is its second run",
    async () => {
      await at("arc/group-failed", "Workflow");

      await expect.element(runGroup(3)).toHaveTextContent("failed at its checks");
      const boundary = runGroup(3).getByRole("region", { name: "Checks at this boundary" });
      await expect.element(boundary).toHaveTextContent("screens_test failed");
      await expect.element(boundary).toHaveTextContent("second run");
      // What the next Drone is given is the Check's own output, not a summary.
      await expect.element(boundary).toHaveTextContent("1 of 1384 failed");
      // One group at a time, named with the group this one is holding back.
      await expect.element(boundary).toHaveTextContent("No task of group 4 starts");
    },
  );

  test(
    "arc/group-failed: the six Checks that passed are drawn beside the one that did not, " +
      "rather than the group reading red with nothing said",
    async () => {
      await at("arc/group-failed", "Workflow");

      const checks = runGroup(3)
        .getByRole("region", { name: "Checks at this boundary" })
        .getByRole("listitem");
      // Every Check the boundary declares has a row, and exactly one is red.
      await expect.poll(() => checks.all().length).toBe(7);
      const reads = () => checks.all().map((one) => one.element().getAttribute("data-reads"));
      expect(reads().filter((one) => one === "failed")).toHaveLength(1);
      expect(reads().filter((one) => one === "passed")).toHaveLength(6);
      await expect.element(checks.filter({ hasText: "typecheck" }).first()).toHaveTextContent("passed");
    },
  );

  test(
    "arc/done-touched: T6 still reads done and carries a flag saying a later task edited the " +
      "file it had finished, and T7 is named as the task that did",
    async () => {
      await at("arc/done-touched", "Workflow");
      // Group three passed, so it is folded — a done task is read by opening it.
      await openGroup(3);

      await expect.element(runTask("T6")).toHaveTextContent("touched later · T7");
      await expect.element(runTask("T6").getByText("Done")).toBeInTheDocument();
      await expect.element(runGroup(3)).toHaveTextContent("passed");
      // And the task that did it is still working, with turns and no cost.
      await expect.element(runTask("T7")).toHaveTextContent("6 turns");
      await expect.element(runTask("T7")).not.toHaveTextContent("$");
    },
  );

  test(
    "arc/executing-sequential: a task opens into what its Drone was told, what it may touch, " +
      "what it runs beside, and a redirect addressed to that task's own Drone",
    async () => {
      await at("arc/executing-sequential", "Workflow");
      await runTask("T5").getByRole("button").first().click();

      const panel = page.getByRole("region", { name: /^T5 · .*, task$/ });
      await expect.element(panel).toBeVisible();
      await expect.element(panel).toHaveTextContent("Its agent is working");
      await expect.element(panel).toHaveTextContent("14 turns");
      await expect.element(panel).toHaveTextContent("Running.tsx");
      await expect
        .element(panel.getByRole("region", { name: "What its Drone was told" }))
        .toHaveTextContent("The panel lists Drones");
      await expect
        .element(panel.getByRole("region", { name: "Redirect" }))
        .toHaveTextContent("Drone on T5");
      // Hold to stop this task, in the same panel as the reading. #1536.
      await expect.element(panel.getByRole("button", { name: /^Hold to/ })).toBeVisible();
    },
  );
});

/** One group of the implement step, on the run, by the heading it carries. */
const runGroup = (ordinal: number) =>
  page
    .getByRole("list", { name: "The groups of this step, in the order they run" })
    .getByRole("listitem")
    .filter({ hasText: new RegExp(`^Group ${ordinal}`) })
    .first();

/** One task's row on the run, by the name the board gives it. */
const runTask = (id: string) => page.getByRole("listitem", { name: new RegExp(`^${id} `) });

/** Open a group that is not the one moving, the way a person opens it. */
async function openGroup(ordinal: number): Promise<void> {
  const head = runGroup(ordinal).getByRole("button", { name: new RegExp(`^Group ${ordinal}`) }).first();
  if (head.element().getAttribute("aria-expanded") === "false") await head.click();
}

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
  test(
    "arc/landed: Land shows the whole test set run again before the pull request was " +
      "offered, with the one case that has no spec named as not covered",
    async () => {
      mount("arc/landed");
      await expect
        .element(page.getByText(/run again at handoff/i).first())
        .toBeVisible();
      for (const spec of [
        "crates/api/src/tests/running.rs",
        "packages/screens/src/overview.test.ts",
        "packages/screens/src/Running.test.tsx",
        "packages/screens/src/Board.test.tsx",
      ]) {
        await expect.element(page.getByText(spec, { exact: true }).first()).toBeVisible();
      }
      // The case with no spec says so in words, and nothing beside it reads as a pass.
      await expect.element(page.getByText("not covered").first()).toBeVisible();
      await expect.element(page.getByText("no spec covers Board.tsx")).toBeVisible();
      // There is no before-run, and the board says so rather than leaving a gap.
      await expect.element(page.getByText(/no before-run/i)).toBeVisible();
    },
  );

  test(
    "arc/landed: the run a person made themselves is drawn beside Fleet's, and says which of " +
      "them ran it",
    async () => {
      mount("arc/landed");
      const byHand = page.getByText(/^Run by hand$/i);
      await expect.element(byHand).toBeVisible();
      // Fleet ran the set; the press this person made is its own set, named for them.
      await expect.element(page.getByText("Fleet").first()).toBeVisible();
      await expect.element(page.getByText("you", { exact: true }).first()).toBeVisible();
    },
  );

  test("arc/landed: the pull request is on screen as an address a press opens", async () => {
    const { api } = mount("arc/landed");
    const opened: string[] = [];
    const was = api.openPullRequest;
    api.openPullRequest = async (jobId: string) => {
      opened.push(jobId);
      return was(jobId);
    };
    // The header carries the same address on its `#1604` link, so the board's
    // row is named by the text it draws rather than by the title both share.
    const address = page.getByText("https://git.example/armada/pull/1604", { exact: true });
    await expect.element(address).toBeVisible();
    await page.getByRole("button", { name: "Open" }).first().click();
    await expect.poll(() => opened).toHaveLength(1);
  });

  test("arc/landed: what it cost counts the agents and the Checks the retry ran again", async () => {
    mount("arc/landed");
    await expect.element(page.getByText("Drones", { exact: true }).first()).toBeVisible();
    // Eight tasks and a retried group of two: ten agents, and its seven Checks twice.
    await expect.element(page.getByText("10", { exact: true }).first()).toBeVisible();
    await expect.element(page.getByText("32", { exact: true }).first()).toBeVisible();
    await expect.element(page.getByText(/group three ran again/)).toBeVisible();
  });

  test("arc/landed: the board says what completes this Job, and what it left behind", async () => {
    mount("arc/landed");
    await expect.element(page.getByText("Completes when its pull request lands.")).toBeVisible();
    await expect.element(page.getByText("Left behind", { exact: true })).toBeVisible();
    await expect
      .element(page.getByText(".armada/worktrees/3-show-what-s-running-in-the-drones-stat"))
      .toBeVisible();
  });
  test("members/stacked: three pull requests are drawn in the order they land — one merged, one waiting on you, one stacked on it — and nothing on screen names a shape", async () => {
    mount("members/stacked");
    const order = page.getByRole("list", { name: "Pull requests, in the order they land" });
    await expect.element(order).toBeVisible();

    const cards = order.getByRole("listitem").elements();
    expect(cards.map((card) => card.getAttribute("aria-label"))).toEqual([
      "1. Give the store one shape",
      "2. Read the store through selectors",
      "3. Drop the store singleton",
    ]);

    // The first is in, the second is a person's to answer, and the third is
    // branched off the second and still working.
    await expect.element(member(1).getByText("Its pull request merged.")).toBeVisible();
    await expect.element(member(2).getByText("awaiting review")).toBeVisible();
    await expect.element(member(3).getByText(/^Stacked on the one before it\./)).toBeVisible();
    // Stacked means its pull request targets the branch before it, and not main.
    await expect
      .element(member(3).getByText("armada/23-read-the-store-through-selectors"))
      .toBeVisible();

    expect(document.body.textContent).not.toMatch(/convoy|train|atomic/i);
  });

  test("members/merged: the third member reads as merged into the one before it and parked as a draft, so nothing is asked of a reviewer yet", async () => {
    mount("members/merged");
    await expect
      .element(page.getByRole("list", { name: "Pull requests, in the order they land" }))
      .toBeVisible();

    await expect.element(member(3).getByText("Parked until the one before it lands.")).toBeVisible();
    // Parked rather than stacked: it targets where the Job lands, and it has
    // opened nothing for anybody to review.
    await expect.element(member(3).getByText("main")).toBeVisible();
    await expect.element(member(3).getByText("None opened yet.")).toBeVisible();
  });

  test("members/stacked: the parent says it is done when every member has landed, which is not the same as its own pull request merging", async () => {
    mount("members/stacked");

    // The second member reached `awaiting_review` and the third is running;
    // one pull request is in, which is what the count says and what a count
    // of finished Jobs would not.
    await expect
      .element(page.getByText("Done when every member has landed. 1 of 3 pull requests merged."))
      .toBeVisible();
  });
});

/** One member's card, by where it sits in the order. */
const member = (ordinal: number) =>
  page
    .getByRole("list", { name: "Pull requests, in the order they land" })
    .getByRole("listitem")
    .nth(ordinal - 1);

describe("the wave", () => {
  it.todo(
    "epic/wave: five Jobs are drawn under one plan with what each has reached, and the two " +
      "that merged are told apart from the three still out",
  );
  it.todo("epic/wave: pressing one of the wave's Jobs opens that Job");
});

describe("one Job per workflow kind", () => {
  // The Workflow tab draws `JobDetail.steps`, which is the workflow the Job
  // froze. So what these claims read is the workflow file, through the screen.
  async function stepsOf(name: string) {
    const scenario = scenarioNamed(name)!;
    mount(scenario);
    await page.getByRole("tab", { name: /^Workflow/ }).click();
    const job = scenario.state.jobs[0]!;
    return scenario.state.holds.workflows.find((one) => one.id === job.workflow_id)!.steps;
  }

  /** The card for one step of the run, by the name and state it carries. */
  const stepCard = (label: string) => page.getByRole("button", { name: new RegExp(`^${label}, `) });

  test.for([
    ["kind/feature", 4],
    ["kind/bug", 3],
    ["kind/revert", 2],
    ["kind/prototype", 3],
    ["kind/refactor", 3],
    ["kind/design-plan", 2],
    ["kind/code-review", 3],
    ["kind/epic", 3],
  ] as const)("%s draws the steps its own workflow file declares", async ([name, count]) => {
    const steps = await stepsOf(name);
    expect(steps, `${name} declares ${String(count)} steps`).toHaveLength(count);
    for (const step of steps) await expect.element(stepCard(step.label).first()).toBeVisible();
    // And no step the boards invented: `design-plan` is draft and present with
    // no `decide`, and only a feature Job has a step called Write tests.
    if (!steps.some((step) => step.label === "Write tests")) {
      expect(stepCard("Write tests").query(), `${name} draws a step it never declared`).toBeNull();
    }
    expect(stepCard("Decide").query(), `${name} draws a step it never declared`).toBeNull();
  });

  test("no Job on the Board runs a workflow called verify-and-ship", async () => {
    const { scenario } = mount("kinds");
    await openBoard();
    expect(scenario.state.holds.workflows.map((one) => one.id)).not.toContain("verify-and-ship");
    expect(document.body.textContent).not.toContain("verify-and-ship");
  });

  test("kind/code-review: the run ends at a step that delivers a review, and offers no diff to land", async () => {
    const steps = await stepsOf("kind/code-review");
    const last = steps[steps.length - 1]!;
    expect(last.label).toBe("Deliver the review");
    await expect.element(stepCard(last.label).first()).toBeVisible();
    // Nothing on this run lands anything: a code review delivers a review.
    expect(page.getByRole("button", { name: /^Merge/ }).query()).toBeNull();
    expect(page.getByRole("button", { name: /^Land/ }).query()).toBeNull();
  });
});
