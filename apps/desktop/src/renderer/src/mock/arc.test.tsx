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
  it.todo(
    "arc/planned: the Plan tab draws four groups in the order they run, with eight tasks " +
      "under them and no task in two groups",
  );
  it.todo(
    "arc/planned: each task names the files it will touch, the tier the planner gave it, and " +
      "the model that tier resolved to",
  );
  it.todo(
    "arc/planned: each group says which Checks run at its end — four where it writes Rust, " +
      "seven where it writes Bridge",
  );
  it.todo(
    "arc/planned: the case that covers a file two groups touch is drawn at the last of them, " +
      "and the case with no spec reads as not covered rather than as passing",
  );
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
