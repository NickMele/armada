// Two moments before the Job exists: typing the prompt, and sketching beside it.
//
// **Nothing here is a Job, and the fixtures on the Board are other people's
// work.** What a person is holding lives in `draft` — the words, the picture,
// the two refs, and what else is running where this would write.
//
// **`gates` is empty and that is the state, not a gap.** The workflow is
// chosen by the proposer, and the gates lock at approval (#1530, 21 Sep), so
// a dispatch moment has a tier map and a cap and no gate rows at all.

import type { ArcMoment } from "./arc-base";
import { ARC_NOW, arcStep, featureWorkflow } from "./arc-base";
import { ARC_TIERS } from "./arc-plan";
import { lightFixture } from "./light";
import type {
  BranchesAnswer,
  LandingRule,
  PeerOverlapAnswer,
  ProposalView,
  SketchAttachment,
} from "../../draft";
import type { JobFixture } from "../fixture";

const PROMPT =
  "The rail says Drones 1 of 2 and nothing says what the 2 is. Make the stat say what is " +
  "running, and let a press on it list the Drone, its Job and its step.";

/**
 * The branches both ref fields pick over: the base, and the two Jobs already
 * writing where this work would. What `branchesOf` derives from a Manifest and
 * the worktrees Fleet holds — no read answers it yet, so the moment carries it.
 */
export const ARC_BRANCHES: BranchesAnswer = [
  { name: "main", base: true },
  {
    name: "armada/18-fold-the-capacity-read",
    base: false,
    job: "Fold the capacity read into one query",
  },
  {
    name: "armada/19-give-the-rail-its-own-scroll",
    base: false,
    job: "Give the rail its own scroll",
  },
];

/** Where the work starts and where it lands. They differ, and both are drawn. */
export const ARC_LANDING: LandingRule = {
  target: "main",
  from_ref: "main",
  prs: "job",
  branching: "job",
  pr_mode: "ready",
  complete_when: "pr_merged",
  land_together: [],
};

/** The tier map and the cap, as the dispatch form holds them. */
export function arcProposal(over: Partial<ProposalView> = {}): ProposalView {
  return {
    status: "proposing",
    title: "Show what is running in the Drones stat",
    gates: [],
    fleet_always_looks: true,
    tiers: ARC_TIERS,
    drone_cap: 2,
    machine_cap: 4,
    from_ref: "main",
    pr_mode: "ready",
    ...over,
  };
}

/** Two Jobs already writing where this one would. */
function peerJobs(): JobFixture[] {
  return [
    lightFixture(
      {
        id: "01M2D3P0QW001CAPACITYREAD",
        handle: "18-fold-the-capacity-read",
        title: "Fold the capacity read into one query",
        status: "running",
        workflow: featureWorkflow(),
        at: "implement",
        steps: [arcStep("plan", "Plan the change", 1), arcStep("implement", "Implement", 2)],
        says: "running — another Job is writing where this one would",
        created_at: "2026-09-22T08:30:00Z",
        started_at: "2026-09-22T08:31:00Z",
        branch: "armada/18-fold-the-capacity-read",
        detail: { write_targets: ["crates/api/src/"] },
      },
      ARC_NOW,
    ),
    lightFixture(
      {
        id: "01M2D3P0QW001RAILSCROLL0",
        handle: "19-give-the-rail-its-own-scroll",
        title: "Give the rail its own scroll",
        status: "awaiting_review",
        workflow: featureWorkflow(),
        at: "handoff",
        steps: [arcStep("plan", "Plan the change", 1), arcStep("handoff", "Review the change", 4)],
        says: "awaiting_review — a second Job holding one of the same files",
        created_at: "2026-09-22T07:40:00Z",
        started_at: "2026-09-22T07:41:00Z",
        branch: "armada/19-give-the-rail-its-own-scroll",
        detail: { write_targets: ["packages/screens/src/"] },
      },
      ARC_NOW,
    ),
  ];
}

/** Who else claims these paths. A fact, never a verdict — nothing is blocked. */
function peers(): PeerOverlapAnswer {
  return {
    paths_asked: ["crates/api/src/", "crates/fleet/src/", "packages/screens/src/"],
    peers: [
      {
        job: "01M2D3P0QW001CAPACITYREAD",
        title: "Fold the capacity read into one query",
        status: "running",
        shared_paths: ["crates/api/src/"],
      },
      {
        job: "01M2D3P0QW001RAILSCROLL0",
        title: "Give the rail its own scroll",
        status: "awaiting_review",
        shared_paths: ["packages/screens/src/overview.ts"],
      },
    ],
  };
}

/**
 * The picture, the Studio node it was made from, and the boxes behind it.
 *
 * **`drawn` is what the composer reopens.** The staged path is the PNG that
 * goes out; a flattened image cannot be edited, so a moment replaying the
 * composer carries what it was drawn from as well (#1547).
 */
function sketch(): SketchAttachment {
  return {
    staged_path: "/Users/user/Library/Application Support/Armada/staged/drones-stat.png",
    width: 720,
    height: 340,
    produced_by: "rail-stats",
    said: "The panel opens under the stat, with the Drone's Job on the first line.",
    drawn: {
      shapes: [
        { id: "b1", x: 0, y: 0, body: "Drones 1 of 2" },
        { id: "b2", x: 0, y: 180, body: "a panel under it, one Drone to a line" },
        { id: "b3", x: 320, y: 180, body: "the Job and the step each Drone is on" },
      ],
      joins: [
        { id: "b1-b2", from: "b1", to: "b2" },
        { id: "b2-b3", from: "b2", to: "b3" },
      ],
    },
  };
}

export function dispatchTyping(): ArcMoment {
  return {
    name: "dispatchTyping",
    says: "Dispatch — the prompt half typed, and what else is writing there",
    fixtures: peerJobs(),
    draft: {
      prompt: PROMPT,
      branches: ARC_BRANCHES,
      peers: peers(),
      proposal: arcProposal(),
      landing: ARC_LANDING,
    },
  };
}

export function dispatchSketch(): ArcMoment {
  return {
    name: "dispatchSketch",
    says: "Dispatch — a sketch attached beside the prompt",
    fixtures: peerJobs(),
    draft: {
      prompt: PROMPT,
      branches: ARC_BRANCHES,
      sketch: sketch(),
      peers: peers(),
      proposal: arcProposal(),
      landing: ARC_LANDING,
    },
  };
}
