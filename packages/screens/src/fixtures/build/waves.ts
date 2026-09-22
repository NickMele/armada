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

import type { JobMembersView, LandingRule, MemberView } from "../../draft";
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
      row: { landed: "merged" },
      detail: {
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
      detail: {
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
    }),
  ];
}

/** What the parent's members are, and how each one's work reaches the next. */
function membersView(third: MemberView["link"]): JobMembersView {
  return {
    job: PARENT_ID,
    title: PARENT_TITLE,
    members: [
      {
        job: MEMBER_IDS.a,
        title: "Give the store one shape",
        status: "completed_success",
        link: "merged",
        landed_at: "2026-09-22T09:40:00Z",
      },
      {
        job: MEMBER_IDS.b,
        title: "Read the store through selectors",
        status: "awaiting_review",
        link: "published",
      },
      { job: MEMBER_IDS.c, title: "Drop the store singleton", status: "running", link: third },
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
      steps: [
        arcStep("plan", "Plan the wave", 1),
        arcStep("dispatch", "Dispatch the wave", 2),
        arcStep("roll_up", "Roll up the wave", 3),
      ],
      says: "running — a wave of Jobs under one plan, three of five still out",
      created_at: "2026-09-22T07:10:00Z",
      started_at: "2026-09-22T07:12:00Z",
    },
    ARC_NOW,
  );
}

/** What the wave dispatched: five Jobs, at four different moments. */
function waveChildren(): { fixture: JobFixture; link: MemberView["link"] }[] {
  const rows: [string, string, string, string, MemberView["link"]][] = [
    ["A", "32-refuse-an-unknown-code", "Refuse an unknown code at the seam", "completed_success", "merged"],
    ["B", "33-name-the-fault-in-the-toast", "Name the fault in the toast", "completed_success", "merged"],
    ["C", "34-carry-the-code-into-the-log", "Carry the code into the journal", "awaiting_review", "published"],
    ["D", "35-say-which-half-refused", "Say which half refused", "running", "stacked"],
    ["E", "36-drop-the-second-error-shape", "Drop the second error shape", "queued", "stacked"],
  ];
  return rows.map(([suffix, handle, title, status, link], at) => ({
    link,
    fixture: lightFixture(
      {
        id: `01M2D8B2YL001WAVE00000${suffix}`,
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
        started_at: status === "queued" ? undefined : "2026-09-22T07:22:00Z",
        ended_at: status === "completed_success" ? "2026-09-22T09:05:00Z" : undefined,
        branch: status === "queued" ? undefined : `armada/${handle}`,
        row: {
          origin: "sub_dispatched",
          dispatched_by: WAVE_ID,
          ...(status === "completed_success" ? { landed: "merged" } : {}),
        },
      },
      ARC_NOW + at,
    ),
  }));
}

export function epicWave(): ArcMoment {
  const children = waveChildren();
  return {
    name: "wave",
    says: "A wave — five Jobs under one plan, two merged and three still out",
    fixtures: [waveParent(), ...children.map((one) => one.fixture)],
    opens: WAVE_ID,
    draft: {
      members: {
        job: WAVE_ID,
        title: "Carry the error contract through every surface",
        members: children.map((one) => ({
          job: one.fixture.job.id,
          title: one.fixture.job.title,
          status: one.fixture.job.status,
          link: one.link,
        })),
      },
      landing: landingInOrder("ready"),
    },
  };
}
