// Three moments that are several Jobs at once: work landing in order, twice,
// and a wave of Jobs under one plan.
//
// **No kind name anywhere** (#1530, 22 Sep). A Job is a Job: the parent here
// holds a plan and one approval, its members are Jobs of their own, and a
// screen built on this says "three pull requests, landing in order" rather
// than naming a shape.
//
// **`dispatched_by` is what makes a member a member** — the wire field
// `jobMembersOf` reads — and how one member's work reaches the next is the
// draft's `MemberLink`, which nothing on the wire says at all.

import type { CommandInFlight, JudgeQuestion, StepDetail } from "@armada/protocol";

import type { JobMembersView, LandingRule, MemberView } from "../../draft";
import type { Outstanding } from "../../outstanding";
import type { JobFixture } from "../fixture";
import type { ArcMoment } from "./arc-base";
import { ARC_NOW, arcStep, featureWorkflow } from "./arc-base";
import { lightFixture } from "./light";
import type { LightJob } from "./light";
import { epicWorkflow } from "./kinds-workflows";

const PARENT_ID = "01M2D7A1XK001SETTINGSPLIT";
const PARENT_TITLE = "Move the settings store out in three landings";

const MEMBER_IDS = {
  a: "01M2D7A1XK001MEMBER00000A",
  b: "01M2D7A1XK001MEMBER00000B",
  c: "01M2D7A1XK001MEMBER00000C",
};

/** One member Job, dispatched by the parent above. */
function member(over: Partial<LightJob> & Pick<LightJob, "id" | "handle" | "title" | "status" | "at" | "says" | "created_at">): JobFixture {
  return lightFixture(
    {
      workflow: featureWorkflow(),
      steps: [
        arcStep("plan", "Plan the change", 1),
        arcStep("implement", "Implement", 2),
        arcStep("tests", "Write tests", 3),
        arcStep("handoff", "Review the change", 4),
      ],
      branch: `armada/${over.handle}`,
      ...over,
      row: { origin: "sub_dispatched", dispatched_by: PARENT_ID, ...over.row },
    },
    ARC_NOW,
  );
}

/** The parent: it holds the plan and the one approval, and lands nothing itself. */
function parent(): JobFixture {
  return lightFixture(
    {
      id: PARENT_ID,
      handle: "21-move-the-settings-store-out",
      title: PARENT_TITLE,
      status: "running",
      workflow: featureWorkflow(),
      at: "implement",
      steps: [
        arcStep("plan", "Plan the change", 1),
        arcStep("implement", "Implement", 2),
        arcStep("tests", "Write tests", 3),
        arcStep("handoff", "Review the change", 4),
      ],
      says: "running — the parent holds the plan, and its members do the landing",
      created_at: "2026-09-22T08:00:00Z",
      started_at: "2026-09-22T08:02:00Z",
      detail: { write_targets: ["packages/settings/src/"] },
    },
    ARC_NOW,
  );
}

/** The three members, in the order they land. */
function members(): JobFixture[] {
  return [
    member({
      id: MEMBER_IDS.a,
      handle: "22-give-the-store-one-shape",
      title: "Give the store one shape",
      status: "completed_success",
      at: "handoff",
      says: "completed_success — the first landing merged",
      created_at: "2026-09-22T08:05:00Z",
      started_at: "2026-09-22T08:06:00Z",
      ended_at: "2026-09-22T09:40:00Z",
      row: { landed: "merged", tasks: { done: 4, working: 0, open: 0, dropped: 0 } },
      detail: {
        write_targets: ["packages/settings/src/store.ts"],
        delivery: {
          commit: "a19c4b7",
          pushed: "origin/armada/22-give-the-store-one-shape",
          pull_request: "https://git.example/armada/pull/1591",
          landed: "merged",
        },
      },
    }),
    member({
      id: MEMBER_IDS.b,
      handle: "23-read-the-store-through-selectors",
      title: "Read the store through selectors",
      status: "awaiting_review",
      at: "handoff",
      says: "awaiting_review — the second landing is waiting on you",
      created_at: "2026-09-22T08:05:00Z",
      started_at: "2026-09-22T09:41:00Z",
      row: { asking: true, tasks: { done: 3, working: 0, open: 0, dropped: 0 } },
      detail: {
        write_targets: ["packages/settings/src/read.ts", "apps/desktop/src/renderer/"],
        judge_question: MEMBER_QUESTION,
        delivery: {
          commit: "c72d0e9",
          pushed: "origin/armada/23-read-the-store-through-selectors",
          pull_request: "https://git.example/armada/pull/1598",
        },
      },
    }),
    member({
      id: MEMBER_IDS.c,
      handle: "24-drop-the-store-singleton",
      title: "Drop the store singleton",
      status: "running",
      at: "implement",
      says: "running — the third landing is working on top of the second",
      created_at: "2026-09-22T08:05:00Z",
      started_at: "2026-09-22T10:20:00Z",
      row: { tasks: { done: 1, working: 1, open: 3, dropped: 0 } },
      detail: { write_targets: ["packages/settings/src/", "crates/config/src/"] },
    }),
  ];
}

/**
 * The refusal the second member is holding, answered from the parent.
 *
 * **The wire's own `JudgeQuestion`**, against the member's Job id — answering
 * it here sends the same `answer_judge` as answering it on that Job, which is
 * the whole reason a person never has to leave this screen to clear it.
 */
const MEMBER_QUESTION = {
  step_id: "handoff",
  criterion_id: "02-selectors-cover-the-empty-store",
  question: "Does every selector answer for a store nothing has written to yet?",
  expected: "A case for the empty store beside each selector",
  produced: "Two of the six selectors are exercised only against a filled store",
  consequence: "A first launch reads undefined through those two, before anything is saved",
  asked_at: "2026-09-22T10:48:00Z",
};

/**
 * What the parent's members are, and how each one's work reaches the one
 * before it.
 *
 * **The first carries no link**, because nothing is before it; its pull
 * request targets where the Job lands and `landed` says that pull request
 * merged. The other two carry a link each, so both moments together draw all
 * three of `docs/concepts/landing.md`'s edges.
 */
function membersView(third: MemberView["link"]): JobMembersView {
  return {
    job: PARENT_ID,
    title: PARENT_TITLE,
    members: [
      {
        job: MEMBER_IDS.a,
        title: "Give the store one shape",
        status: "completed_success",
        landed: true,
        landed_at: "2026-09-22T09:40:00Z",
        branch: "armada/22-give-the-store-one-shape",
        pull_request: "https://git.example/armada/pull/1591",
        scope: ["packages/settings/src/store.ts"],
        tasks: { done: 4, working: 0, open: 0, dropped: 0 },
      },
      {
        // It waits on the snapshot the first member's merge publishes, not on
        // the merge itself — the `published` edge, and the only one of the
        // three that needs nothing at the parent's level.
        job: MEMBER_IDS.b,
        title: "Read the store through selectors",
        status: "awaiting_review",
        link: "published",
        landed: false,
        branch: "armada/23-read-the-store-through-selectors",
        pull_request: "https://git.example/armada/pull/1598",
        scope: ["packages/settings/src/read.ts", "apps/desktop/src/renderer/"],
        tasks: { done: 3, working: 0, open: 0, dropped: 0 },
        question: MEMBER_QUESTION,
      },
      {
        job: MEMBER_IDS.c,
        title: "Drop the store singleton",
        status: "running",
        link: third,
        landed: false,
        branch: "armada/24-drop-the-store-singleton",
        scope: ["packages/settings/src/", "crates/config/src/"],
        tasks: { done: 1, working: 1, open: 3, dropped: 0 },
      },
    ],
  };
}

/** One pull request per member, and the parent is done when all of them land. */
function landingInOrder(prMode: LandingRule["pr_mode"]): LandingRule {
  return {
    target: "main",
    from_ref: "main",
    prs: "group",
    branching: "group",
    pr_mode: prMode,
    complete_when: "all_members_landed",
    land_together: [],
  };
}

export function membersStacked(): ArcMoment {
  return {
    name: "stacked",
    says: "Three pull requests landing in order — the third is stacked on the second",
    fixtures: [parent(), ...members()],
    opens: PARENT_ID,
    draft: { members: membersView("stacked"), landing: landingInOrder("ready") },
  };
}

export function membersMerged(): ArcMoment {
  return {
    name: "merged",
    // Parked: the third member's work is merged into the one before it and its
    // own pull request is a draft, so nothing is asked of a reviewer yet.
    says: "Three pull requests landing in order — the third is merged into the second and parked",
    fixtures: [parent(), ...members()],
    opens: PARENT_ID,
    draft: { members: membersView("merged"), landing: landingInOrder("draft") },
  };
}

const WAVE_ID = "01M2D8B2YL001ERRORCONTRACT";

/**
 * The step that recorded the split, with the Judge `epic.json` puts on it —
 * two criteria, both met on the pass being run now.
 */
function planStep(): StepDetail {
  const step = arcStep("plan", "Plan the wave", 1);
  return {
    ...step,
    state: "advanced",
    // `epic.json` declares it, and it is what says which step recorded the
    // split — `tab-plan.tsx` reads the check, never the workflow's name.
    checks: [{ kind: "plan_recorded" }],
    judged: [
      { attempt: 2, criterion_id: "draws_the_split", verdict: "met" },
      { attempt: 2, criterion_id: "each_piece_carries_its_own_brief", verdict: "met" },
    ],
    attempts: [{ attempt: 2, outcome: "advanced", started_at: "2026-09-22T07:12:00Z", ended_at: "2026-09-22T07:19:00Z" }],
    verdicts: [{ attempt: 2, named: "passed" }],
  };
}

/**
 * The step the loop returns through. `pass` and `verdict_routing_target` are
 * what draw the iteration count, and both are on the wire — `epic.json` caps
 * the loop at five.
 */
function rollUpStep(): StepDetail {
  return {
    ...arcStep("roll_up", "Roll up the wave", 3),
    pass: { number: 2, of: 5 },
    verdict_routing_target: "plan",
  };
}

/** The wave's parent, on the three steps `epic.json` declares. */
export function waveParent(): JobFixture {
  return lightFixture(
    {
      id: WAVE_ID,
      handle: "31-carry-the-error-contract-everywhere",
      title: "Carry the error contract through every surface",
      status: "running",
      workflow: epicWorkflow(),
      at: "dispatch",
      steps: [planStep(), arcStep("dispatch", "Dispatch the wave", 2), rollUpStep()],
      says: "running — a wave of Jobs under one plan, three of five still out",
      created_at: "2026-09-22T07:10:00Z",
      started_at: "2026-09-22T07:12:00Z",
    },
    ARC_NOW,
  );
}

const WAVE_IDS = {
  a: "01M2D8B2YL001WAVE00000A",
  b: "01M2D8B2YL001WAVE00000B",
  c: "01M2D8B2YL001WAVE00000C",
  d: "01M2D8B2YL001WAVE00000D",
  e: "01M2D8B2YL001WAVE00000E",
};

/**
 * The refusal the Judge opened on the Job at the gate, and the one that
 * stopped the Job holding a Drone. Both are `JobDetail.judge_question`, which
 * is what the existing card is drawn from.
 */
const GATE_REFUSAL: JudgeQuestion = {
  step_id: "handoff",
  criterion_id: "every_code_reaches_the_journal",
  question: "Does every refusal the seam produces reach the journal with its code?",
  expected: "A refusal with an unknown code is journalled as `error.unknown`, with the raw code beside it.",
  produced: "An unknown code is journalled with an empty `code` field and the raw value is dropped.",
  consequence: "A person reading the journal after an unknown refusal cannot tell which code arrived.",
  asked_at: "2026-09-22T10:48:00Z",
};

const BLOCKED_REFUSAL: JudgeQuestion = {
  step_id: "implement",
  criterion_id: "the_half_is_named",
  question: "Does the message say which half refused — Bridge or Fleet?",
  expected: "A transport failure names the side that refused, as `error-contract.md` requires.",
  produced: "The message reads `the request failed` and names neither side.",
  consequence: "A person cannot tell whether to restart Fleet or reopen the window.",
  asked_at: "2026-09-22T11:02:00Z",
};

/** The command the last Job's Drone stopped inside, which the Manifest has not cleared. */
const UNCLEARED: CommandInFlight = {
  call: "call_gh_api_1",
  step_id: "implement",
  asked_at: "2026-09-22T11:11:00Z",
  tool: "Bash",
  detail: "gh api repos/:owner/:repo/issues/1544/comments",
  truncated: false,
  length: 48,
  offers: ["allow_for_job", "always_allow", "reject"],
  rules: ["gh", "gh api"],
  suggested_rule: "gh api",
};

/**
 * What the wave dispatched: five Jobs, and which waits on which.
 *
 * **The order is the work's own.** The seam refuses first; the two surfaces
 * that carry the code follow it; naming which half refused follows the toast
 * that would say so; and the second error shape can only be dropped once every
 * surface carries the first — so it waits on both, and never the reverse.
 *
 * **No landing link, because a wave is not a landing order.** One of these
 * waits on another because its work depends on that work, and each lands when
 * it is done — nothing says the fourth merges after the third.
 */
function waveChildren(): { fixture: JobFixture; waits: string[] }[] {
  const rows: [string, string, string, string, string[]][] = [
    [WAVE_IDS.a, "32-refuse-an-unknown-code", "Refuse an unknown code at the seam", "completed_success", []],
    [WAVE_IDS.b, "33-name-the-fault-in-the-toast", "Name the fault in the toast", "completed_success", [WAVE_IDS.a]],
    [WAVE_IDS.c, "34-carry-the-code-into-the-log", "Carry the code into the journal", "awaiting_review", [WAVE_IDS.a]],
    [WAVE_IDS.d, "35-say-which-half-refused", "Say which half refused", "escalated", [WAVE_IDS.b]],
    [WAVE_IDS.e, "36-drop-the-second-error-shape", "Drop the second error shape", "running", [WAVE_IDS.c, WAVE_IDS.d]],
  ];
  return rows.map(([id, handle, title, status, waits], at) => ({
    waits,
    fixture: lightFixture(
      {
        id,
        handle,
        title,
        status,
        workflow: featureWorkflow(),
        at: status === "completed_success" || status === "awaiting_review" ? "handoff" : "implement",
        steps: [
          arcStep("plan", "Plan the change", 1),
          arcStep("implement", "Implement", 2),
          arcStep("tests", "Write tests", 3),
          arcStep("handoff", "Review the change", 4),
        ],
        says: `${status} — one Job of the wave`,
        created_at: "2026-09-22T07:20:00Z",
        started_at: "2026-09-22T07:22:00Z",
        ended_at: status === "completed_success" ? "2026-09-22T09:05:00Z" : undefined,
        branch: `armada/${handle}`,
        row: {
          origin: "sub_dispatched",
          dispatched_by: WAVE_ID,
          ...(status === "completed_success" ? { landed: "merged" } : {}),
          // The flag that lifts a row into Needs you, and what tells a running
          // Job with a Drone inside a call apart from one simply working.
          ...(id === WAVE_IDS.e ? { asking: true } : {}),
        },
        detail: {
          ...(id === WAVE_IDS.c ? { judge_question: GATE_REFUSAL } : {}),
          ...(id === WAVE_IDS.d ? { judge_question: BLOCKED_REFUSAL } : {}),
          ...(id === WAVE_IDS.e ? { command_waiting: UNCLEARED, when_blocked: "ask_me" } : {}),
        },
      },
      ARC_NOW + at,
    ),
  }));
}

/** The three questions the wave is holding open, as main gathers them. */
function waveQuestions(): Outstanding[] {
  return [
    { kind: "judge", job_id: WAVE_IDS.c, question: GATE_REFUSAL },
    { kind: "judge", job_id: WAVE_IDS.d, question: BLOCKED_REFUSAL },
    { kind: "command", job_id: WAVE_IDS.e, waiting: UNCLEARED },
  ];
}

export function epicWave(): ArcMoment {
  const children = waveChildren();
  return {
    name: "wave",
    says: "A wave — five Jobs under one plan, two merged and three still out",
    fixtures: [waveParent(), ...children.map((one) => one.fixture)],
    opens: WAVE_ID,
    questions: waveQuestions(),
    draft: {
      wave: {
        job: WAVE_ID,
        title: "Carry the error contract through every surface",
        // Pass 1 split the work at the seam alone and was replaced when the
        // roll-up sent it back; pass 2 is the plan being run now. A loop return
        // replaces `plan.md` whole, so the first is history.
        rounds: [
          { round: 1, says: "The seam alone — one Job to refuse an unknown code", live: false },
          { round: 2, says: "The seam, then every surface that reads a refusal", live: true },
        ],
        jobs: children.map((one) => ({
          job: one.fixture.job.id,
          title: one.fixture.job.title,
          status: one.fixture.job.status,
          handle: one.fixture.job.handle,
          round: 2,
          waits_on: one.waits,
          ...(one.fixture.job.landed === undefined ? {} : { landed: one.fixture.job.landed }),
        })),
      },
    },
  };
}
