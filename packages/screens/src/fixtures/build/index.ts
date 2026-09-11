// Every fixture, named as the sentence it renders — one Job, eighteen moments.
//
// **The order is the roster's**, tier one first: `running`, `awaiting_review`
// and `escalated`/`gate_failure` are full depth, and everything after them is
// lighter. `build.test.ts` walks `FIXTURES` rather than importing each name,
// so a fixture added here is a fixture the proof covers without a second edit.

import { running, runningWaitingOnACommand } from "./running";
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
import { retryingCheckFailure, runningAtGate } from "./gating";
import { reviewAtDelivery } from "./delivering";

export {
  running,
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
};

export const FIXTURES = [
  running(),
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
];
