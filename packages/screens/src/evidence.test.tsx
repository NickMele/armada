// The Checks and the Verdicts, on the screen Bridge actually composes.
//
// # This file exists because the drawing and the app had drifted
//
// Both chapters existed as components and as story fixtures for a while, and
// neither was ever built into the screen that ships: `chapters.tsx` returned
// three chapters and the Storybook story returned five, three of them from the
// real builder and two of them invented. A person looking at a real refused Job
// saw no Checks and no Verdicts at all, and the story that would have shown the
// gap was drawing its own data instead of the app's.
//
// **So this mounts `chaptersOf`.** Nothing here writes a `StepChapter`. Every
// row asserted below is built by the same call `JobDetail.tsx` makes, from a
// `StepDetail` shaped like the wire's — which is the only arrangement in which
// "the story draws it" and "the app draws it" cannot come apart again.
//
// **A browser test and not a Storybook story, because of the layer rule.**
// `xtask/src/rules_layers.rs` puts `@armada/components` below `@armada/screens`
// and refuses an import the other way, so no story can reach `chaptersOf`. This
// package's browser project is where a screen is mounted — `vitest.config.ts`
// says so, and says why.

import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import { InsideAJob } from "./InsideAJob";
import type { Artifact, JobDetail, JobSummary, StepDetail } from "@armada/protocol";

import { chaptersOf } from "./chapters";
import { NO_FRAMES } from "./frames";
import { CHECKS_CHAPTER } from "./checks";
import { headingOf } from "./heading";
import { mount, unmount } from "./mounted";
import type { Outputs } from "./outputs";
import { phasesOf, type Opens } from "./phases";
import { renderFor } from "./render";
import { runOf } from "./run";

afterEach(unmount);

/**
 * The story's chapters, by the ordinal each one draws.
 *
 * **The number is what a reader navigates by**, so a story that drew 1, 2, 3, 5
 * is the defect — and it is the one a conditional chapter walks into. Read off
 * the mark rather than off the builder's answer, because the mark is what is on
 * the screen.
 */
function ordinals(): (string | undefined)[] {
  return [...document.querySelectorAll(".armada-story__chapter")].map((chapter) =>
    chapter.textContent?.slice(0, 1),
  );
}

const JOB_ID = "01M130Y1380016YK5S0JXBXDQ5";

/** Now, fixed. An elapsed is read on this screen, so it has to be a number. */
const NOW = Date.parse("2026-09-09T10:00:00Z");

/** The Job the panel is showing — refused by its Judge, so it is stopped. */
function job(over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: JOB_ID,
    handle: "12-a-job",
    title: "Coalesce concurrent token refreshes",
    status: "escalated",
    workflow_id: "bug",
    owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-09-09T09:00:00Z",
    current_step_id: "verify",
    assigned_drone: "01M1HHJ6XB001BZJZ4BE2XKY34",
    ...over,
  };
}

/** The step, as `crates/ipc/src/detail.rs` serves one. */
function step(over: Partial<StepDetail> = {}): StepDetail {
  return {
    step_id: "verify",
    label: "Verify",
    ordinal: 3,
    state: "stopped",
    check_runs: [],
    overridden: false,
    judged: [],
    flagged: [],
    attempts: [{ attempt: 1, outcome: "refused", started_at: "2026-09-09T09:30:00Z" }],
    verdicts: [],
    entered_at: "2026-09-09T09:30:00Z",
    updated_at: "2026-09-09T09:41:00Z",
    ...over,
  };
}

/** A step whose Checks passed and whose panel of three refused one criterion. */
function refusedStep(): StepDetail {
  return step({
    checks: [
      { kind: "manifest_check", name: "check:test_suite", run: "cargo test --workspace" },
      { kind: "manifest_check", name: "check:public_api", run: "cargo public-api" },
    ],
    check_runs: [
      {
        attempt: 1,
        name: "check:test_suite",
        outcome: "passed",
        output_path: ".armada/checks/01M130/test_suite.log",
      },
      { attempt: 1, name: "check:public_api", outcome: "passed" },
    ],
    judge_checks: [{ criteria: 2, panel_size: 3, gaming_check: false }],
    judged: [
      { attempt: 1, criterion_id: "c1", member: 1, verdict: "met", given: HANDED_C1 },
      { attempt: 1, criterion_id: "c1", member: 2, verdict: "met", given: HANDED_C1 },
      { attempt: 1, criterion_id: "c1", member: 3, verdict: "met", given: HANDED_C1 },
      {
        attempt: 1,
        criterion_id: "c2",
        member: 1,
        verdict: "not_met",
        expected: "Suite red when a loose-format date with trailing spaces is parsed",
        produced: "Suite green, with that case deleted from tests/loose.rs",
        consequence: "A parser regression ships as verified",
        brief_path: ".armada/judge/01M130/c2.md",
        // Both refusers quoted the same lines of the same Check's output,
        // which is why the grounds group into one block above.
        cited: [{ region: "check:test_suite", from_line: 2007, to_line: 2008 }],
        given: HANDED_C2,
      },
      { attempt: 1, criterion_id: "c2", member: 2, verdict: "met", given: HANDED_C2 },
      {
        attempt: 1,
        criterion_id: "c2",
        member: 3,
        verdict: "not_met",
        expected: "Suite red when a loose-format date with trailing spaces is parsed",
        produced: "Suite green, with that case deleted from tests/loose.rs",
        consequence: "A parser regression ships as verified",
        brief_path: ".armada/judge/01M130/c2.md",
        cited: [{ region: "check:test_suite", from_line: 2007, to_line: 2008 }],
        given: HANDED_C2,
      },
    ],
  });
}

/**
 * What each criterion's panel was handed. **One per criterion, not one per
 * step**: every criterion is its own brief, so two criteria digest two ways and
 * a fixture that shared one would assert a comparison Fleet never makes.
 */
const HANDED_C1 = { digest: "3f7a10c2b40de991", size: 41_204, model: "haiku" };
const HANDED_C2 = { digest: "9c41ab0e77d25631", size: 43_880, model: "haiku" };

/** The Job's frozen criteria, which is what gives a verdict row its words. */
const CRITERIA = [
  { criterion_id: "c1", text: "The public API is byte-identical.", source: "brief" },
  {
    criterion_id: "c2",
    text: "Behaviour is unchanged for every input the previous implementation accepted.",
    source: "brief",
  },
];

/** What was asked for, so a press that opens nothing is a failing test. */
let asked: Artifact[] = [];

function opens(): Opens {
  return {
    jobId: JOB_ID,
    open: async (_jobId, what) => {
      asked.push(what);
      return { ok: true };
    },
    onSaid: () => {},
  };
}

/**
 * The screen, composed the way `JobDetail.tsx` composes it.
 *
 * **Every region comes from a reading in this package** — `headingOf`, `runOf`,
 * `phasesOf`, `chaptersOf` — against one `JobDetail` payload. Nothing below
 * hands `InsideAJob` a chapter it wrote itself, which is the whole point of the
 * file.
 */
function screen(
  showing: StepDetail,
  criteria = CRITERIA,
  outputs?: Outputs,
  /**
   * Which chapter is open on mount. **Absent is the ordinary screen**, where
   * every chapter shows its preview and none its content — so a test asserting
   * on a chapter's body says which one it opened, the way a person would.
   */
  opening?: string,
  /** Fleet's own reason the gate could not decide, for the `gate_undecided` cases. */
  undecided?: string,
): void {
  asked = [];
  const summary = job();
  const whole: JobDetail = {
    job: summary,
    created_at: summary.created_at,
    steps: [showing],
    acceptance_criteria: criteria,
    dependencies: [],
  };
  const records = opens();
  const heading = headingOf({
    job: summary,
    whole,
    workflow: undefined,
    now: NOW,
    render: renderFor(summary),
    stale: false,
    acting: false,
    approving: false,
    reporting: false,
    onReporting: () => {},
    onAct: () => {},
    onApprove: () => {},
    onReport: async () => ({ ok: true }),
    onOpenPullRequest: async () => ({ ok: true }),
    onCopied: () => {},
    onSaid: () => {},
    // The way into the Job settings panel. Inert here, for the reason the six
    // below are: nothing in this file asserts about it.
    onOpenSettings: () => {},
    // The six the budget work added to `Heading`, three per ceiling. This file
    // draws a heading to reach the evidence beneath it and asserts nothing
    // about raising a cap, so they are inert here.
    onRaiseCap: () => {},
    raising: false,
    onRaising: () => {},
    onRaiseTurnCap: () => {},
    raisingTurns: false,
    onRaisingTurns: () => {},
  });
  if (heading === null) throw new Error("this Job draws no heading, so there is no screen");
  mount(
    <InsideAJob
      heading={heading}
      run={runOf(whole, NOW, showing.step_id, [])}
      step={{
        label: showing.label,
        fields: [],
        ...(opening === undefined ? {} : { openChapter: opening }),
        phases: phasesOf(showing, criteria, records, summary.status),
        chapters: chaptersOf({
          job: summary,
          step: showing,
          steps: [showing],
          criteria,
          frames: NO_FRAMES,
          watching: { rows: [], skipped: 0 },
          footprint: { state: "none" },
          kept: undefined,
          diff: { state: "none" },
          live: false,
          transcript: undefined,
          log: (region) => ({ region, openId: null, onOpen: () => {} }),
          calls: { of: () => undefined, fetch: () => {} },
          outputs: outputs ?? { of: () => undefined, fetch: () => {} },
          sheet: null,
          opens: records,
          onOpenSheet: () => {},
          now: NOW,
          following: { reading: { state: "none" }, picked: null, pick: () => {}, follow: () => {} },
          undecided,
        }),
      }}
    />,
  );
}

test("a judged step draws both evidence chapters, after Produced", async () => {
  screen(refusedStep());
  // The ordinals are what a reader navigates by, so they are asserted as the
  // numbers the story draws rather than as the mere presence of a heading.
  await expect.element(page.getByText("Checks", { exact: true })).toBeVisible();
  await expect.element(page.getByText("Verdicts", { exact: true })).toBeVisible();
  expect(ordinals()).toEqual(["1", "2", "3", "4", "5"]);
});

test("the Checks chapter says what each declared Check came to", async () => {
  screen(refusedStep());
  // Exact, because the phase strip above names the same Check — which is the
  // point: one reading, two surfaces.
  await expect.element(page.getByText("check:test_suite", { exact: true })).toBeVisible();
  await expect.element(page.getByText("check:public_api", { exact: true })).toBeVisible();
  // The tier's own sentence, and the chapter's header — one call in
  // `gates.ts`, two surfaces, so they cannot come to different counts.
  await expect.element(page.getByText("2 of 2 passed").first()).toBeVisible();
  expect(page.getByText("2 of 2 passed").elements().length).toBe(2);
  // And the chapter opens onto more than its preview, which is what makes the
  // reading below reachable rather than only built.
  await expect
    .element(page.getByText("Read what the Checks asserted and printed"))
    .toBeVisible();
});

test("the Judge's row on the Checks list counts criteria, never calls", async () => {
  // Two criteria answered by three judges is six `judged` rows, and a row that
  // counted those would report one refusal out of six.
  screen(refusedStep());
  // Twice on purpose — the Judge's row and the Verdicts chapter's own header,
  // from one call. Two counts that could disagree is what `gates.ts` prevents.
  await expect.element(page.getByText("1 of 2 criteria refused").first()).toBeVisible();
  expect(page.getByText("1 of 2 criteria refused").elements().length).toBe(2);
  await expect.element(page.getByText("judge · 2 criteria · panel of 3")).toBeVisible();
});

test("a Check's output opens the file the wire named", async () => {
  screen(refusedStep());
  await page.getByRole("button", { name: "test_suite.log" }).first().click();
  expect(asked).toContainEqual({ kept: ".armada/checks/01M130/test_suite.log", what: "check" });
});

test("the assertion set tells a Check that was skipped from one that failed", async () => {
  // **The finding the set exists for**, in Armada's own words:
  // `check-outcomes.toml` says a step that advanced having skipped every Check
  // verified nothing, and the record must be able to say so. `skipped` advances
  // and is not a pass, so it is `absent` here and never `passed`.
  screen(
    step({
      checks: [
        { kind: "manifest_check", name: "check:test_suite", run: "cargo test --workspace" },
        { kind: "manifest_check", name: "check:browser", run: "pnpm test:browser" },
      ],
      check_runs: [
        { attempt: 1, name: "check:test_suite", outcome: "passed" },
        {
          attempt: 1,
          name: "check:browser",
          outcome: "skipped",
          produced: "no changed file is under packages/",
        },
      ],
    }),
    CRITERIA,
    undefined,
    CHECKS_CHAPTER,
  );
  await expect.element(page.getByText("What the suite asserted")).toBeVisible();
  const rows = [...document.querySelectorAll(".armada-assertions__row")];
  expect(rows.map((row) => row.getAttribute("data-named"))).toEqual(["passed", "absent"]);
  // The skip carries the record's own reason, which is the first thing a reader
  // asks about a Check that did not run.
  await expect.element(page.getByText("no changed file is under packages/")).toBeVisible();
});

test("a Check compares against its own previous attempt, never against a commit", async () => {
  // Nothing runs a step's Checks at the commit the work branched from, so the
  // comparison served is between runs of the step. A Check that passed on
  // attempt 1 and was skipped on attempt 2 is a case that stopped existing, and
  // it is the one this column is for.
  screen(
    step({
      attempts: [
        { attempt: 1, outcome: "refused", started_at: "2026-09-09T09:00:00Z" },
        { attempt: 2, outcome: "refused", started_at: "2026-09-09T09:30:00Z" },
      ],
      checks: [{ kind: "manifest_check", name: "check:test_suite", run: "cargo test" }],
      check_runs: [
        { attempt: 1, name: "check:test_suite", outcome: "passed" },
        {
          attempt: 2,
          name: "check:test_suite",
          outcome: "skipped",
          produced: "no changed file is under crates/",
        },
      ],
    }),
    CRITERIA,
    undefined,
    CHECKS_CHAPTER,
  );
  await expect.element(page.getByText("passed at attempt 1 · not run at attempt 2")).toBeVisible();
});

test("a Check's output is read into the chapter, not only opened elsewhere", async () => {
  // **The audit happens on the screen the Check is on.** The row has carried
  // `output_path` since the file did, and everything Bridge could do with it
  // was hand it to the operating system.
  screen(refusedStep(), CRITERIA, {
    of: () => ({
      state: "got",
      output: {
        attempt: 1,
        name: "check:test_suite",
        path: ".armada/checks/01M130/test_suite.log",
        lines: ["--- stdout ---", "test parses_rfc3339_offset ... ok"],
        from_line: 1_939,
        total_lines: 2_180,
        bytes: 61_204,
        whole: false,
      },
    }),
    fetch: () => {},
  }, CHECKS_CHAPTER);
  await expect.element(page.getByText("test parses_rfc3339_offset ... ok")).toBeVisible();
  // The file's own numbering, not the window's: a reader citing 1,939 means the
  // file's 1,939th line.
  await expect.element(page.getByText("1939")).toBeVisible();
  // A truncated reading says it is truncated, and says whose output it is.
  await expect
    .element(page.getByText("check:test_suite · lines 1,939–1,940 of 2,180"))
    .toBeVisible();
});

test("a Check that recorded no output draws no reader", async () => {
  // Not an empty console frame. A built-in assertion runs no command and a
  // Check that never started printed nothing, and a pane offering to read
  // either is a control that opens onto nothing.
  screen(
    step({
      checks: [{ kind: "diff_nonempty" }],
      check_runs: [{ attempt: 1, name: "diff_nonempty", outcome: "passed" }],
    }),
    CRITERIA,
    undefined,
    CHECKS_CHAPTER,
  );
  await expect.element(page.getByText("What the suite asserted")).toBeVisible();
  expect(document.querySelectorAll(".armada-console").length).toBe(0);
});

test("the verdict grid draws one row per criterion and one mark per judge", async () => {
  screen(refusedStep());
  // Two criteria, three judges each — six `judged` rows arrive and two rows are
  // drawn, which is the whole of the panel grouping.
  await expect.element(page.getByText("The public API is byte-identical.")).toBeVisible();
  const marks = document.querySelectorAll(".armada-judge-verdicts__mark");
  expect(marks.length).toBe(6);
  expect([...marks].filter((mark) => mark.getAttribute("data-mark") === "not_met").length).toBe(2);
});

test("one veto refuses the criterion, and the split says how many", async () => {
  screen(refusedStep());
  await expect.element(page.getByText("refused by 2 of 3")).toBeVisible();
  await expect.element(page.getByText("2 of 3 judges refused, on the same grounds")).toBeVisible();
  // The two judges wrote the same finding, so it is drawn once.
  expect(document.querySelectorAll(".armada-judge-refusal").length).toBe(1);
});

test("the refusal draws the three fields a refusal owes", async () => {
  screen(refusedStep());
  await expect
    .element(page.getByText("Suite green, with that case deleted from tests/loose.rs"))
    .toBeVisible();
  await expect.element(page.getByText("A parser regression ships as verified")).toBeVisible();
});

test("the brief the verdict answers opens from the refusal", async () => {
  screen(refusedStep());
  await page.getByRole("button", { name: /c2\.md/ }).first().click();
  expect(asked).toContainEqual({ kept: ".armada/judge/01M130/c2.md", what: "brief" });
});

test("a step that declares no Check and asks no Judge draws neither chapter", async () => {
  // The rule this screen keeps everywhere: an empty labelled region reads as a
  // value that failed to load, so an ungated step gets three chapters and the
  // phase strip's sentence — not two empty ones.
  screen(step({ checks: [], judge_checks: [] }), []);
  await expect.element(page.getByText("Produced", { exact: true })).toBeVisible();
  expect(ordinals()).toEqual(["1", "2", "3"]);
  expect(document.body.textContent).not.toContain("Verdicts");
});

test("a step that gates on a Judge alone draws the Verdicts chapter as chapter four", async () => {
  screen(
    step({
      checks: [],
      judge_checks: [{ criteria: 1, gaming_check: false }],
      judged: [{ attempt: 1, criterion_id: "c1", verdict: "met" }],
    }),
    [CRITERIA[0] as (typeof CRITERIA)[number]],
  );
  await expect.element(page.getByText("Verdicts", { exact: true })).toBeVisible();
  expect(ordinals()).toEqual(["1", "2", "3", "4"]);
  // A panel of one is silent about being a panel — `Judged.member` is absent at
  // `panel_size: 1`, and the column says what it is rather than `j1`.
  await expect.element(page.getByText("Judge", { exact: true })).toBeVisible();
});

test("a Judge that has not been asked says what is coming, not nothing", async () => {
  screen(
    step({
      state: "running",
      checks: [],
      judge_checks: [{ criteria: 2, panel_size: 3, gaming_check: false }],
    }),
  );
  await expect
    .element(page.getByText(/The 2 criteria it will be asked were frozen/))
    .toBeVisible();
  // The chapter's own header says where the panel stands, in the registry's
  // word — the same one the strip's tier stands at.
  expect(page.getByText("not reached").elements().length).toBeGreaterThan(0);
});

test("the citation list says which judge quoted what, and where in the brief", async () => {
  screen(refusedStep());
  // `j1 · 02` — the member and the criterion's frozen position, which is what a
  // citation names. Two refusers, two rows, and the criterion nothing objected
  // to contributes none: a `met` answer writes no prose to quote from.
  await expect.element(page.getByText("j1 \u00b7 02")).toBeVisible();
  await expect.element(page.getByText("j3 \u00b7 02")).toBeVisible();
  expect(document.querySelectorAll(".armada-judge-citations__row").length).toBe(2);
  expect(page.getByText("check:test_suite lines 2007\u20132008").elements().length).toBe(2);
});

test("a citation opens the brief it points into", async () => {
  screen(refusedStep());
  await page.getByRole("button", { name: /j1 \u00b7 02/ }).first().click();
  expect(asked).toContainEqual({ kept: ".armada/judge/01M130/c2.md", what: "brief" });
});

test("what each judge was handed is drawn, and says the panel was a panel", async () => {
  screen(refusedStep());
  // The digest of the object every member of c2's panel was sent, in the
  // comparison view — which is the one that opens.
  await expect.element(page.getByText("9c41ab0e77d25631")).toBeVisible();
  await expect.element(page.getByText("Every judge was handed the same brief")).toBeVisible();
  // A segment per member, plus the comparison — which is the reading the view
  // exists for, and the reason it leads.
  await expect.element(page.getByRole("tab", { name: "Compare" })).toBeVisible();
  expect(page.getByRole("tab").elements().length).toBe(4);
});

test("a panel handed two different objects says so instead", async () => {
  // The answer this view exists to be able to give. Nothing in Fleet writes it
  // today — the brief is built once outside the panel loop — which is exactly
  // why the digest is stored per member rather than asserted off that loop.
  const apart = refusedStep();
  const drifted = { ...HANDED_C2, digest: "0000000000000000", size: 12 };
  screen({
    ...apart,
    judged: apart.judged.map((one) =>
      one.criterion_id === "c2" && one.member === 3 ? { ...one, given: drifted } : one,
    ),
  });
  await expect.element(page.getByText("The judges were not handed the same brief")).toBeVisible();
  expect(document.body.textContent).not.toContain("Every judge was handed the same brief");
});

test("one judge draws what it cited and no panel comparison at all", async () => {
  screen(
    step({
      checks: [],
      judge_checks: [{ criteria: 1, gaming_check: false }],
      judged: [
        {
          attempt: 1,
          criterion_id: "c1",
          verdict: "not_met",
          expected: "The exported signatures unchanged",
          produced: "One argument added to `parse`",
          consequence: "Every caller outside this repository stops compiling",
          brief_path: ".armada/judge/01M130/c1.md",
          cited: [{ region: "diff", from_line: 88, to_line: 88 }],
          given: HANDED_C1,
        },
      ],
    }),
    [CRITERIA[0] as (typeof CRITERIA)[number]],
  );
  // The criterion alone, with no judge in front of it. `member` is absent at a
  // panel of one, and naming it `j1` would invent a position the record
  // deliberately does not carry.
  await expect.element(page.getByText("diff line 88")).toBeVisible();
  expect(document.body.textContent).not.toContain("j1 \u00b7 ");
  // What the one judge was handed is still drawn, and with no segmented
  // control: a control offering one segment is a promise nobody kept.
  await expect.element(page.getByText("What the Judge was handed")).toBeVisible();
  expect(page.getByRole("tab").elements().length).toBe(0);
  // And no assurance, because there is no panel for the objects to be
  // identical across.
  expect(document.body.textContent).not.toContain("Every judge was handed the same brief");
});

test("a panel that refused nothing draws no citation list", async () => {
  // A no-objection writes no prose under the veto-only contract, so there is
  // nothing to have quoted — and an empty list under every passing step would
  // be the screen explaining its own contract to somebody who did not ask.
  screen(
    step({
      checks: [],
      judge_checks: [{ criteria: 1, gaming_check: false }],
      judged: [{ attempt: 1, criterion_id: "c1", verdict: "met", given: HANDED_C1 }],
    }),
    [CRITERIA[0] as (typeof CRITERIA)[number]],
  );
  await expect.element(page.getByText("Verdicts", { exact: true })).toBeVisible();
  expect(document.body.textContent).not.toContain("What the panel quoted");
});

/**
 * The tests step of `2-refuse-a-merge-press-whose-chosen-comments` — `#633`'s
 * own case: every Check passed, one criterion was declared and the Judge
 * timed out before the panel answered any of it.
 */
function undecidedStep(): StepDetail {
  return step({
    checks: [
      { kind: "every_manifest_check" },
      { kind: "manifest_check", name: "check:build", run: "cargo build" },
      { kind: "manifest_check", name: "check:test", run: "cargo test" },
    ],
    check_runs: [
      { attempt: 1, name: "check:build", outcome: "passed" },
      { attempt: 1, name: "check:test", outcome: "passed" },
    ],
    judge_checks: [{ criteria: 1, gaming_check: false }],
    judged: [],
    attempts: [
      { attempt: 1, outcome: "stopped", why: "gate_undecided", started_at: "2026-09-09T09:30:00Z" },
    ],
    verdicts: [{ attempt: 1, named: "failed", trigger: "gate_undecided" }],
    last_verdict: { attempt: 1, named: "failed", trigger: "gate_undecided" },
  });
}

/** Fleet's own sentence, as it crosses the wire — lower-case, unpunctuated. */
const UNDECIDED_RAW = "the Judge did not answer inside its budget";

/** The same sentence, in this screen's own case. */
const UNDECIDED_SAID = "The Judge did not answer inside its budget.";

test("the sweep marker's row is never counted or drawn among the Checks", async () => {
  // Collapsed, so the chapter shows its `CheckRuns` preview — the region a
  // marker row with no run would draw into — rather than the assertion set
  // opening it would replace.
  screen(undecidedStep(), CRITERIA);
  await expect.element(page.getByText("2 of 2 passed").first()).toBeVisible();
  await expect.element(page.getByText("check:build").first()).toBeVisible();
  expect(document.body.textContent).not.toContain("every_manifest_check");
  expect(document.body.textContent).not.toContain("Nothing has run this Check yet.");
});

test("the Judge's row draws Fleet's own reason, sentence-cased, and not doubled", async () => {
  screen(undecidedStep(), CRITERIA, undefined, undefined, UNDECIDED_RAW);
  // Twice: the Judge's row on the Checks list and the Verdicts chapter's own
  // preview, from the one reading `gates.ts` gives both.
  await expect.element(page.getByText(UNDECIDED_SAID, { exact: false }).first()).toBeVisible();
  expect(page.getByText(UNDECIDED_SAID, { exact: false }).elements().length).toBe(2);
  // Fleet's sentence stands alone — it is not said twice over.
  expect(document.body.textContent).not.toContain("did not answer. The Judge");
  expect(document.body.textContent).not.toContain("Waiting for every Check to finish.");
});

test("the Verdicts chapter's short value is the registry's word, the sentence is in the preview", async () => {
  screen(undecidedStep(), CRITERIA, undefined, undefined, UNDECIDED_RAW);
  await expect.element(page.getByText("could not read the artifact")).toBeVisible();
  expect(document.body.textContent).not.toContain("has not been asked anything on this step yet");
});

test("a Bridge behind the Fleet that sent it still says the panel did not answer", async () => {
  // `undecided` absent is a Fleet built before the field, or a step this
  // reader opened that `stuck` is not about — either way the shorter sentence
  // stays true.
  screen(undecidedStep(), CRITERIA);
  await expect
    .element(page.getByText("The panel was asked and did not answer.", { exact: false }).first())
    .toBeVisible();
  expect(
    page.getByText("The panel was asked and did not answer.", { exact: false }).elements().length,
  ).toBe(2);
  expect(document.body.textContent).not.toContain(UNDECIDED_RAW);
});

test("a Fleet that served neither reading draws the grid and nothing under it", async () => {
  // Every row a Fleet before protocol 8.3 wrote. The grid is unchanged and the
  // two regions below it are absent — never an empty digest table, which would
  // read as a panel that was handed nothing.
  const older = refusedStep();
  screen({
    ...older,
    judged: older.judged.map(({ cited: _cited, given: _given, ...rest }) => rest),
  });
  await expect.element(page.getByText("refused by 2 of 3")).toBeVisible();
  expect(document.body.textContent).not.toContain("What the panel quoted");
  expect(document.body.textContent).not.toContain("What each judge was handed");
});
