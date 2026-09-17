// Which acts the step header draws on a job that is still working.
//
// # Why this is a browser test and not a unit test
//
// `steering.test.ts` proves the reading; this proves the reading reaches a
// control. Those went out of step for a whole milestone — #145 made a redirect
// legal on a healthy drone, `keys.ts` already sent a person to job detail with
// `d` on exactly that row, and job detail drew nothing there — so the thing
// worth pinning is that a person watching a drone work can see the act, which
// is a question about the rendered tree and not about a function's return.
//
// The step's own acts are asserted here and the job's are not: the two kills
// are `Acts.tsx`'s, unchanged, and drawn from the same pointer they always were.

import { afterEach, expect, test } from "vitest";
import { page } from "vitest/browser";

import type { JobDetail as JobWhole, JobSummary, StepDetail, Stuck } from "@armada/protocol";
import { ACT_LABEL } from "./copy";
import { mount, rerender, unmount } from "./mounted";
import type { ActAnswer, ActingAct } from "./pending";
import { StepActs } from "./StepActs";

afterEach(unmount);

/** A job that is running, with a drone on it. */
function job(over: Partial<JobSummary> = {}): JobSummary {
  return {
    id: "01M130Y1380016YK5S0JXBXDQ5",
    handle: "12-a-job",
    title: "Coalesce concurrent token refreshes",
    status: "running",
    workflow_id: "bug",
    owner_manifest_id: "01M1CNPKTV0018H2M1CXDNBK06",
    origin: "dispatched",
    urgency: "normal",
    atomic: false,
    model: "sonnet",
    created_at: "2026-08-31T09:00:00Z",
    branch: "armada/01M130Y1380016YK5S0JXBXDQ5",
    assigned_drone: "01M1D0X0000016YK5S0JXBXDQ5",
    ...over,
  };
}

/** The step header of a job that is working, with no detail read yet. */
function working(summary: JobSummary): void {
  mount(
    <StepActs
      job={summary}
      whole={null}
      // Nothing on a working job opens a record; the override dialog is the
      // one surface under here that does, and it is not drawn on this render.
      opens={{ jobId: summary.id, open: async () => ({ ok: true }), onSaid: () => {} }}
      render="working"
      acting={false}
      rerunningChecks={false}
      stale={false}
      onAct={() => {}}
      onRedirect={() => {}}
      onOverrule={() => {}}
      onRerun={() => {}}
      onRerunChecks={() => {}}
    />,
  );
}

test("a person watching a drone work is offered the redirect", async () => {
  working(job());
  await expect
    .element(page.getByRole("button", { name: ACT_LABEL.redirect }))
    .toBeInTheDocument();
});

test("a working job with no drone on it is offered nothing", async () => {
  // There is no session for the instruction to go into, and nothing on the
  // wire holds a note for a running job — so the control is absent rather
  // than refused on the press.
  working(job({ assigned_drone: undefined }));
  expect(page.getByRole("button").elements()).toEqual([]);
});

/** A step that stopped on a failed Check, `#1105`'s fixture. */
function checkStoppedStep(): StepDetail {
  return {
    step_id: "verify",
    label: "Verify",
    ordinal: 3,
    state: "stopped",
    check_runs: [],
    overridden: false,
    judged: [],
    flagged: [],
    attempts: [],
    verdicts: [],
    entered_at: "2026-09-14T09:00:00Z",
    updated_at: "2026-09-14T09:10:00Z",
  };
}

function awaitingRepair(): JobWhole {
  const stuck: Stuck = {
    stopped_by: "gate_failure",
    step_id: "verify",
    recourse: ["rerun_checks", "restart_step", "redispatch_job"],
    worktree_on_disk: true,
    drone_unheard: false,
    refused: [],
    refusals: 0,
  };
  return {
    job: job({ status: "awaiting_repair", assigned_drone: undefined }),
    created_at: "2026-09-14T09:00:00Z",
    steps: [checkStoppedStep()],
    acceptance_criteria: [],
    dependencies: [],
    stuck,
  };
}

/** The stopped step header, on a Job a failed Check left at `awaiting_repair`. */
function stopped(
  rerunningChecks: boolean,
  acting = rerunningChecks,
  actingAct?: ActingAct,
  answered?: ActAnswer,
  put: typeof mount = mount,
): void {
  const whole = awaitingRepair();
  put(
    <StepActs
      job={whole.job}
      whole={whole}
      opens={{ jobId: whole.job.id, open: async () => ({ ok: true }), onSaid: () => {} }}
      render="stopped"
      acting={acting}
      actingAct={actingAct}
      answered={answered}
      rerunningChecks={rerunningChecks}
      stale={false}
      onAct={() => {}}
      onRedirect={() => {}}
      onOverrule={() => {}}
      onRerun={() => {}}
      onRerunChecks={() => {}}
    />,
  );
}

test("a step stopped on a failed check is offered run checks again", async () => {
  stopped(false);
  await expect
    .element(page.getByRole("button", { name: ACT_LABEL.rerun_checks }))
    .toBeInTheDocument();
});

test("the press says the Checks are running while its own request is out", async () => {
  stopped(true);
  // Pending, not disabled: the control that sent the press stays focusable —
  // `aria-busy` carries the wait, `toBeDisabled` would be `Button`'s sibling
  // reading rather than this one's own. #1117.
  await expect
    .element(page.getByRole("button", { name: "Running Checks" }))
    .toHaveAttribute("aria-busy", "true");
});

// `awaitingRepair`'s own fixture offers `rerun_checks` and `restart_step`
// together — `recovery.ts` offers a Checks re-run beside whichever resume act
// applies rather than replacing it — so one fixture proves both readings:
// each button waits on its own name and nothing else's.

test("run checks again is not busy for a different act, and stays disabled", async () => {
  stopped(false, true, "restart_step");
  const button = page.getByRole("button", { name: ACT_LABEL.rerun_checks });
  await expect.element(button).not.toHaveAttribute("aria-busy", "true");
  await expect.element(button).toBeDisabled();
});

test("restart step is busy while its own act is out", async () => {
  stopped(false, true, "restart_step");
  await expect
    .element(page.getByRole("button", { name: "Restarting the step…" }))
    .toHaveAttribute("aria-busy", "true");
});

test("restart step is not busy for a different act, and stays disabled", async () => {
  stopped(false, true, "rerun_checks");
  const restart = page.getByRole("button", { name: ACT_LABEL.restart_step });
  await expect.element(restart).not.toHaveAttribute("aria-busy");
  await expect.element(restart).toBeDisabled();
});

// Fleet's answer lands on the edge of the control that sent the act, and on no
// sibling. `data-answer` is read because no accessible property carries the
// line — it is decoration, and the words of a refusal are drawn elsewhere.

test("a refused restart answers on restart alone", async () => {
  stopped(false, false, undefined, { act: "restart_step", answer: "refused" });
  await expect
    .element(page.getByRole("button", { name: ACT_LABEL.restart_step }))
    .toHaveAttribute("data-answer", "refused");
  await expect
    .element(page.getByRole("button", { name: ACT_LABEL.rerun_checks }))
    .not.toHaveAttribute("data-answer");
});

test("an accepted Checks re-run answers accepted, and the next press clears it", async () => {
  stopped(false, false, undefined, { act: "rerun_checks", answer: "accepted" });
  const rerun = page.getByRole("button", { name: ACT_LABEL.rerun_checks });
  await expect.element(rerun).toHaveAttribute("data-answer", "accepted");
  // The app drops the answer as the next act goes out, and that act waits.
  stopped(false, true, "rerun_checks", undefined, rerender);
  await expect.element(rerun).toHaveAttribute("aria-busy", "true");
  await expect.element(rerun).not.toHaveAttribute("data-answer");
});
