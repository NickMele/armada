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
import { InsideAJob } from "@armada/components";
import type { Artifact, JobDetail, JobSummary, StepDetail } from "@armada/protocol";

import { chaptersOf } from "./chapters";
import { headingOf } from "./heading";
import { mount, unmount } from "./mounted";
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
      { attempt: 1, criterion_id: "c1", member: 1, verdict: "met" },
      { attempt: 1, criterion_id: "c1", member: 2, verdict: "met" },
      { attempt: 1, criterion_id: "c1", member: 3, verdict: "met" },
      {
        attempt: 1,
        criterion_id: "c2",
        member: 1,
        verdict: "not_met",
        expected: "Suite red when a loose-format date with trailing spaces is parsed",
        produced: "Suite green, with that case deleted from tests/loose.rs",
        consequence: "A parser regression ships as verified",
        brief_path: ".armada/judge/01M130/c2.md",
      },
      { attempt: 1, criterion_id: "c2", member: 2, verdict: "met" },
      {
        attempt: 1,
        criterion_id: "c2",
        member: 3,
        verdict: "not_met",
        expected: "Suite red when a loose-format date with trailing spaces is parsed",
        produced: "Suite green, with that case deleted from tests/loose.rs",
        consequence: "A parser regression ships as verified",
        brief_path: ".armada/judge/01M130/c2.md",
      },
    ],
  });
}

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
function screen(showing: StepDetail, criteria = CRITERIA): void {
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
    // The header's cost-cap dialog, which no assertion here reaches. Required
    // by `Heading` and so supplied, rather than left for the compiler to find.
    onRaiseCap: () => {},
    raising: false,
    onRaising: () => {},
    onOpenPullRequest: async () => ({ ok: true }),
    onCopied: () => {},
    onSaid: () => {},
  });
  if (heading === null) throw new Error("this Job draws no heading, so there is no screen");
  mount(
    <InsideAJob
      heading={heading}
      run={runOf(whole, NOW, showing.step_id, [])}
      step={{
        label: showing.label,
        fields: [],
        phases: phasesOf(showing, criteria, records, summary.status),
        chapters: chaptersOf({
          job: summary,
          step: showing,
          criteria,
          render: renderFor(summary),
          watching: { rows: [], skipped: 0 },
          footprint: { state: "none" },
          kept: undefined,
          diff: { state: "none" },
          live: false,
          transcript: undefined,
          log: (region) => ({ region, openId: null, onOpen: () => {} }),
          calls: { of: () => undefined, fetch: () => {} },
          sheet: null,
          opens: records,
          onOpenSheet: () => {},
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
