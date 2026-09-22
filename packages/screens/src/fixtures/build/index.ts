// Every fixture, named as the sentence it renders — one Job, eighteen moments.
//
// **Three rosters, not one.** `FIXTURES` is the Bug Job at every state a Job
// can be listed in. `ARC_MOMENTS` is the Feature Job the new boards were drawn
// against, and a moment there can be several Jobs or none. `KIND_FIXTURES` is
// one Job per workflow kind. Each is walked by its own proof, so a fixture
// added to any of them is covered without a second edit.
//
// **The order is the roster's**, tier one first: `running`, `awaiting_review`
// and `escalated`/`gate_failure` are full depth, and everything after them is
// lighter. `build.test.ts` walks `FIXTURES` rather than importing each name,
// so a fixture added here is a fixture the proof covers without a second edit.

import { running, runningWaitingOnACommand } from "./running";
import { workingAPlan } from "./working-a-plan";
import { review } from "./review";
import { escalatedGateFailure } from "./escalated";
import { escalatedEvidenceSuspect } from "./escalated-judge";
import {
  awaitingApproval,
  awaitingAttestation,
  awaitingRepair,
  piloted,
  queued,
} from "./waiting";
import {
  escalatedBlockedByPolicy,
  escalatedInterrupted,
  escalatedLoopCap,
  escalatedNoReport,
  escalatedSilent,
} from "./escalated-light";
import {
  completedFailed,
  completedSuccess,
  killed,
  rejected,
  superseded,
} from "./terminal";
import { preparing } from "./preparing";
import { reading } from "./reading";
import { unreadable } from "./unreadable";
import { gateChecksStreaming, retryingCheckFailure, runningAtGate } from "./gating";
import { reviewAtDelivery } from "./delivering";

export {
  running,
  workingAPlan,
  runningWaitingOnACommand,
  review,
  escalatedGateFailure,
  escalatedEvidenceSuspect,
  reviewAtDelivery,
  queued,
  awaitingApproval,
  awaitingRepair,
  awaitingAttestation,
  piloted,
  escalatedBlockedByPolicy,
  escalatedInterrupted,
  escalatedSilent,
  escalatedLoopCap,
  escalatedNoReport,
  completedSuccess,
  completedFailed,
  rejected,
  killed,
  superseded,
  preparing,
  reading,
  unreadable,
  retryingCheckFailure,
  runningAtGate,
  gateChecksStreaming,
};

export const FIXTURES = [
  running(),
  workingAPlan(),
  review(),
  escalatedGateFailure(),
  escalatedEvidenceSuspect(),
  reviewAtDelivery(),
  queued(),
  awaitingApproval(),
  awaitingRepair(),
  awaitingAttestation(),
  piloted(),
  escalatedBlockedByPolicy(),
  escalatedInterrupted(),
  escalatedSilent(),
  escalatedLoopCap(),
  escalatedNoReport(),
  completedSuccess(),
  completedFailed(),
  rejected(),
  killed(),
  superseded(),
  preparing(),
  reading(),
  unreadable(),
  retryingCheckFailure(),
  runningAtGate(),
  runningWaitingOnACommand(),
  gateChecksStreaming(),
];

export { ARC_MOMENTS, arcMoment } from "./arc";
export type { ArcDraft, ArcMoment } from "./arc";
export { KIND_FIXTURES, KIND_NAMES } from "./kinds";
export { epicWave, membersMerged, membersStacked } from "./waves";
