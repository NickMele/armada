// One Job per workflow kind, each drawing the steps its own file declares.
//
// **Eight, and no ninth** (owner, 22 Sep 2026): feature, bug, refactor,
// revert, prototype, design-plan, code-review and epic. `verify-and-ship` is
// not one of them — no file declares it, and none is added here.
//
// **The feature Job is the arc's own** and the epic Job is the wave's parent,
// rather than two more strangers: a reader who has walked either recognises
// the Job when it turns up in this roster.
//
// The seven light ones are the Jobs around a moment, not moments themselves —
// `light.ts` says what that depth is and why.

import type { JobFixture } from "../fixture";
import { ARC_NOW, BRIDGE_CHECKS, RUST_CHECKS } from "./arc-base";
import { executingSequential } from "./arc-executing";
import {
  bugWorkflow,
  codeReviewWorkflow,
  designPlanWorkflow,
  prototypeWorkflow,
  refactorWorkflow,
  revertWorkflow,
} from "./kinds-workflows";
import { lightFixture, stepsOf } from "./light";
import { waveParent } from "./waves";

/** The Job on the `bug` workflow: three steps, not six. */
export function bugKind(): JobFixture {
  const workflow = bugWorkflow(BRIDGE_CHECKS);
  return lightFixture(
    {
      id: "01M2D9C3ZM001RECONNECTBAN",
      handle: "41-stop-the-reconnect-banner-sticking",
      title: "Stop the reconnect banner sticking after a resync",
      status: "running",
      workflow,
      at: "implement",
      steps: stepsOf(workflow, "implement", "running", "2026-09-22T10:02:00Z"),
      says: "running — a bug Job, on the three steps its own file declares",
      created_at: "2026-09-22T09:58:00Z",
      started_at: "2026-09-22T10:00:00Z",
      branch: "armada/41-stop-the-reconnect-banner-sticking",
      detail: { write_targets: ["apps/desktop/src/renderer/src/"] },
    },
    ARC_NOW,
  );
}

export function refactorKind(): JobFixture {
  const workflow = refactorWorkflow(BRIDGE_CHECKS);
  return lightFixture(
    {
      id: "01M2D9C3ZM001MANIFESTREAD",
      handle: "42-split-the-manifest-reader-out",
      title: "Split the manifest reader out of the watcher",
      status: "awaiting_review",
      workflow,
      at: "handoff",
      steps: stepsOf(workflow, "handoff", "awaiting_human", "2026-09-22T10:40:00Z"),
      says: "awaiting_review — a refactor Job at its own human gate",
      created_at: "2026-09-22T09:10:00Z",
      started_at: "2026-09-22T09:12:00Z",
      branch: "armada/42-split-the-manifest-reader-out",
      detail: {
        delivery: {
          commit: "d38b0c1",
          pushed: "origin/armada/42-split-the-manifest-reader-out",
          pull_request: "https://git.example/armada/pull/1600",
        },
      },
    },
    ARC_NOW,
  );
}

export function revertKind(): JobFixture {
  const workflow = revertWorkflow(RUST_CHECKS);
  return lightFixture(
    {
      id: "01M2D9C3ZM001JOURNALFLUSH",
      handle: "43-undo-the-journal-flush-change",
      title: "Undo the change to the journal's flush",
      status: "running",
      workflow,
      at: "revert",
      steps: stepsOf(workflow, "revert", "running", "2026-09-22T11:01:00Z"),
      says: "running — a revert Job, two steps and nothing else",
      created_at: "2026-09-22T11:00:00Z",
      started_at: "2026-09-22T11:01:00Z",
      branch: "armada/43-undo-the-journal-flush-change",
      detail: { write_targets: ["crates/fleet/src/journal.rs"] },
    },
    ARC_NOW,
  );
}

export function prototypeKind(): JobFixture {
  const workflow = prototypeWorkflow();
  return lightFixture(
    {
      id: "01M2D9C3ZM001STACKEDRUNPR",
      handle: "44-try-a-stacked-run-beside-the-canvas",
      title: "Try a stacked run beside the canvas",
      status: "awaiting_review",
      workflow,
      at: "build",
      steps: stepsOf(workflow, "build", "awaiting_human", "2026-09-22T10:20:00Z"),
      says: "awaiting_review — a prototype Job, held at Build for a person to look",
      created_at: "2026-09-22T10:05:00Z",
      started_at: "2026-09-22T10:06:00Z",
      branch: "armada/44-try-a-stacked-run-beside-the-canvas",
    },
    ARC_NOW,
  );
}

export function designPlanKind(): JobFixture {
  const workflow = designPlanWorkflow();
  return lightFixture(
    {
      id: "01M2D9C3ZM001HELDTOWHAT00",
      handle: "45-plan-how-a-job-says-what-it-is-held-to",
      title: "Plan how a Job says what it is held to",
      status: "awaiting_review",
      workflow,
      at: "present",
      steps: stepsOf(workflow, "present", "awaiting_human", "2026-09-22T09:50:00Z"),
      says: "awaiting_review — a design-plan Job, presenting what it drafted",
      created_at: "2026-09-22T09:30:00Z",
      started_at: "2026-09-22T09:31:00Z",
    },
    ARC_NOW,
  );
}

export function codeReviewKind(): JobFixture {
  const workflow = codeReviewWorkflow();
  return lightFixture(
    {
      id: "01M2D9C3ZM001MERGELINELOO",
      handle: "46-read-the-merge-line-s-retry-loop",
      title: "Read the merge line's retry loop",
      status: "awaiting_review",
      workflow,
      at: "assess",
      steps: stepsOf(workflow, "assess", "awaiting_human", "2026-09-22T10:55:00Z"),
      says: "awaiting_review — a code-review Job, which delivers a review and no diff",
      created_at: "2026-09-22T10:50:00Z",
      started_at: "2026-09-22T10:51:00Z",
    },
    ARC_NOW,
  );
}

/** The feature Job is the arc's own, mid-`implement`. */
export function featureKind(): JobFixture {
  return executingSequential().fixtures[0]!;
}

/** The epic Job is the wave's parent, mid-`dispatch`. */
export function epicKind(): JobFixture {
  return waveParent();
}

/** One Job per kind, in the order `.armada/workflows/` is read in. */
export const KIND_FIXTURES: readonly JobFixture[] = [
  featureKind(),
  bugKind(),
  refactorKind(),
  revertKind(),
  prototypeKind(),
  designPlanKind(),
  codeReviewKind(),
  epicKind(),
];

/** The workflow each kind Job runs, by the Job's own id. */
export const KIND_NAMES: readonly string[] = [
  "feature",
  "bug",
  "refactor",
  "revert",
  "prototype",
  "design-plan",
  "code-review",
  "epic",
];
