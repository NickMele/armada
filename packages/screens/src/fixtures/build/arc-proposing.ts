// Three moments around the approval gate: the proposer reading, the answer on
// screen, and the Job frozen at the press.
//
// **The wire status is `awaiting_approval` and the word is `classifying`.**
// No registry declares a `classifying` status — `words.ts` carries the word as
// a draft and `job-statuses.toml` is what the badge reads, so the fixture
// carries the status a Fleet would send and the draft carries the word.
//
// **What locks, locks at approval** (#1530, 21 Sep): the gates and the tier
// map are editable up to the press and frozen after it. `approvedFrozen` is
// the moment after, and it is the one that says the issue has moved since.

import type { GateView, LedgerRow, ProposalView } from "../../draft";
import type { ArcMoment } from "./arc-base";
import {
  ARC_APPROVED_AT,
  ARC_JOB_ID,
  ARC_NOW,
  arcCriterionViews,
  arcDetail,
  arcJob,
  arcManifests,
  arcResources,
  arcSteps,
  arcWatched,
  featureWorkflow,
} from "./arc-base";
import { ARC_LANDING, arcProposal, dispatchTyping } from "./arc-dispatch";
import type { JobFixture } from "../fixture";

/**
 * How long the proposer has been out, and how long it may take.
 *
 * Past `PROPOSAL_IS_SLOW`, so the surface is at the register where it asks
 * whether to keep waiting rather than the one where it only says it is
 * reading. The budget is Fleet's own provisional ten minutes.
 */
const READING_FOR_MS = 150_000;
const PROPOSER_BUDGET_MS = 600_000;

/**
 * What each step is gated by, as the proposer proposed it.
 *
 * `handoff` is the fourth state: the repository decides (#1530, 22 Sep). The
 * other three are what `feature.json` declares — a Judge on every step, and
 * Checks wherever the step writes.
 */
function gates(overridden: boolean): GateView[] {
  const handoff: GateView = {
    step_id: "handoff",
    checks: false,
    judge: false,
    you: overridden,
    repository_decides: "review_gate",
    overridden,
  };
  return [
    { step_id: "plan", checks: false, judge: true, you: false },
    { step_id: "implement", checks: true, judge: true, you: false },
    { step_id: "tests", checks: true, judge: true, you: false },
    handoff,
  ];
}

/** The proposal on screen, with a workflow chosen and gates to read. */
function classified(over: Partial<ProposalView> = {}): ProposalView {
  return arcProposal({ status: "awaiting_approval", gates: gates(false), ...over });
}

/**
 * The same proposal one press later, which every moment after it reads.
 *
 * **One spelling for the whole arc past the press.** What locks, locks at
 * approval (#1530, 21 Sep) — so a Job with a plan recorded or a Drone out is
 * held to exactly these gates, this tier map and this cap, and `approved_at` is
 * what draws them as a reading rather than as controls somebody could move
 * under a running Drone.
 */
export function arcApproved(): ProposalView {
  return classified({
    status: "approved",
    gates: gates(true),
    approved_at: ARC_APPROVED_AT,
  });
}

/** The Job itself, at the gate — no branch, no Drone, no step entered. */
function atTheGate(status: string, over = {}): JobFixture {
  const job = arcJob(status, { current_step_id: "plan", branch: undefined, ...over });
  const whole = arcDetail(job, arcSteps(), { branch: undefined });
  return {
    name: `${status} — the proposal is on screen and nothing has run`,
    job,
    watched: arcWatched(whole),
    workflows: [featureWorkflow()],
    manifests: arcManifests(),
    observed: { state: "none" },
    journalled: {
      state: "watching",
      jobId: ARC_JOB_ID,
      log: {
        skipped: 0,
        notes: [
          {
            at: "2026-09-22T09:08:00Z",
            by: "fleet",
            level: "info",
            seq: 1,
            msg: "The proposer read the linked issue and chose the feature workflow.",
          },
        ],
      },
    },
    resources: { state: "read", jobId: ARC_JOB_ID, resources: arcResources("none") },
    recorded: {
      footprint: { state: "none" },
      handed: { state: "none" },
      evidence: { state: "none" },
      diff: { state: "none" },
      remarks: { state: "none" },
    },
    calls: {},
    checkOutputs: {},
    frames: {},
    now: ARC_NOW,
  };
}

/** What the Record holds before anything ran: one press, and what it froze. */
function frozenRecord(): LedgerRow[] {
  return [
    {
      at: "2026-09-22T09:06:00Z",
      coord: null,
      actor: "fleet",
      kind: "proposed",
      what: "the proposer read armada/1162 and chose the feature workflow",
      outcome: "four steps, a Judge on each",
      cursor: 1,
    },
    {
      at: ARC_APPROVED_AT,
      coord: null,
      actor: "person",
      kind: "status_queued",
      what: "approved the dispatch",
      outcome: "the workflow, the gates and the tier map are frozen",
      cursor: 2,
    },
  ];
}

export function proposingReading(): ArcMoment {
  const before = dispatchTyping();
  return {
    name: "proposingReading",
    says: "Classifying — the proposer is still reading, and the form is waiting on it",
    fixtures: before.fixtures,
    proposing: {
      proposal_id: "01M2D3ZF41001PROPOSAL001",
      client_ref: "bridge-1",
      model: "sonnet",
      // Against the real clock, not the arc's: the wait is drawn from the
      // window's own `now`, so a fixed instant here would read as a call that
      // went out months ago. `working-a-plan.ts` moves its instants for the
      // same reason.
      since: new Date(Date.now() - READING_FOR_MS).toISOString(),
      budget_ms: PROPOSER_BUDGET_MS,
      reached: "thinking",
      thinking_tokens: 1_840,
    },
    draft: { ...before.draft, proposal: arcProposal({ status: "proposing" }) },
  };
}

export function proposingReview(): ArcMoment {
  const before = dispatchTyping();
  return {
    name: "proposingReview",
    says: "Classifying — a workflow, four gate rows and a tier map, all still editable",
    fixtures: [atTheGate("awaiting_approval"), ...before.fixtures],
    opens: ARC_JOB_ID,
    draft: {
      prompt: before.draft.prompt,
      proposal: classified(),
      landing: ARC_LANDING,
      criteria: arcCriterionViews(),
    },
  };
}

export function approvedFrozen(): ArcMoment {
  const job = atTheGate("queued", { started_at: ARC_APPROVED_AT, queued_reason: "waiting_on_resources" });
  return {
    name: "approvedFrozen",
    says: "Approved — frozen at the press, and the issue has been edited since",
    fixtures: [{ ...job, name: "queued — approved, and the linked issue has moved since" }],
    opens: ARC_JOB_ID,
    draft: {
      proposal: arcApproved(),
      landing: ARC_LANDING,
      // The Job keeps the words it froze, and says the issue has moved since
      // (#1530, 22 Sep). The instant is the issue's edit, never the freeze.
      criteria: arcCriterionViews("2026-09-22T10:02:00Z"),
      record: frozenRecord(),
    },
  };
}
