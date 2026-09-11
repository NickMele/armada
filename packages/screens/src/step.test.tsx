// The band above a stopped step's story, on the Job `#633` was filed from — a
// tests step that stopped `gate_undecided`: every Check passed and the Judge
// timed out.
//
// **A browser test, not a plain one**, because `noticeOf`'s `title` is JSX —
// the headline and the fact it names are two nodes in one fragment, and a
// plain assertion on the returned object would compare React elements rather
// than what a reader sees.

import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";
import type { JobDetail as JobWhole, JobSummary, StepDetail, Stuck } from "@armada/protocol";

import { mount, unmount } from "./mounted";
import type { Opens } from "./phases";
import { noticeOf } from "./step";

afterEach(unmount);

const JOB_ID = "01M22TYSAE0023MADDP5ZQEYGW";

function opens(): Opens {
  return { jobId: JOB_ID, open: async () => ({ ok: true }), onSaid: () => {} };
}

function job(): JobSummary {
  return {
    id: JOB_ID,
    handle: "2-refuse-a-merge-press-whose-chosen-comments",
    title: "Refuse a merge press whose chosen comments",
    status: "escalated",
    reason: { named: "gate_undecided" },
    workflow_id: "feature",
    owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-09-11T09:00:00Z",
    current_step_id: "tests",
  };
}

/** Every Check passed, so the step's own record names no failed Check. */
function step(): StepDetail {
  return {
    step_id: "tests",
    label: "Tests",
    ordinal: 2,
    state: "stopped",
    checks: [{ kind: "manifest_check", name: "build", run: "cargo build" }],
    check_runs: [{ attempt: 1, name: "build", outcome: "passed" }],
    judge_checks: [{ criteria: 1, gaming_check: false }],
    overridden: false,
    judged: [],
    flagged: [],
    attempts: [
      { attempt: 1, outcome: "stopped", why: "gate_undecided", started_at: "2026-09-11T09:30:00Z" },
    ],
    verdicts: [{ attempt: 1, named: "failed", trigger: "gate_undecided" }],
    entered_at: "2026-09-11T09:30:00Z",
    updated_at: "2026-09-11T09:41:00Z",
  };
}

/** Fleet's own sentence, as it crosses the wire — lower-case, unpunctuated. */
const UNDECIDED_RAW = "the Judge did not answer inside its budget";

/** The same sentence, in this screen's own case, for the band to draw. */
const UNDECIDED_SAID = "The Judge did not answer inside its budget.";

function stuck(showing: StepDetail, over: Partial<Stuck> = {}): Stuck {
  return {
    stopped_by: "gate_undecided",
    undecided: UNDECIDED_RAW,
    step_id: showing.step_id,
    recourse: ["rerun_gate"],
    worktree_on_disk: true,
    drone_unheard: false,
    refused: [],
    refusals: 0,
    ...over,
  };
}

function whole(showing: StepDetail, over: Partial<Stuck> = {}): JobWhole {
  return {
    job: job(),
    created_at: "2026-09-11T09:00:00Z",
    steps: [showing],
    acceptance_criteria: [],
    dependencies: [],
    stuck: stuck(showing, over),
  };
}

test("the band names the trigger and Fleet's own reason for it, sentence-cased", async () => {
  const showing = step();
  const notice = noticeOf(job(), whole(showing), "stopped", showing, opens());
  if (notice === undefined) throw new Error("a stopped render draws a notice");
  mount(
    <>
      {notice.title}
      {notice.children}
    </>,
  );
  // The trigger's own verb, from the registry — never invented here.
  await expect.element(page.getByText("the gate could not decide", { exact: false })).toBeVisible();
  // Which step, and Fleet's own sentence for why — the fact #633 found missing
  // from the screen entirely. Fleet sent it lower-case and unpunctuated; the
  // band draws it as a sentence.
  await expect.element(page.getByText("stopped at Tests", { exact: false })).toBeVisible();
  await expect.element(page.getByText(UNDECIDED_SAID, { exact: false })).toBeVisible();
});

test("no `undecided` sentence where the wire sent none", async () => {
  const showing = step();
  const bare = whole(showing, { undecided: undefined });
  const notice = noticeOf(job(), bare, "stopped", showing, opens());
  if (notice === undefined) throw new Error("a stopped render draws a notice");
  mount(
    <>
      {notice.title}
      {notice.children}
    </>,
  );
  await expect.element(page.getByText("the gate could not decide", { exact: false })).toBeVisible();
  expect(document.body.textContent).not.toContain(UNDECIDED_RAW);
  expect(document.body.textContent).not.toContain(UNDECIDED_SAID);
});
