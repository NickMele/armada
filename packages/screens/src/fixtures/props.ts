// Every prop `JobDetail` needs, answered from one fixture — for the stories that
// draw a fixture and for the proof that renders every one of them.
//
// **One place, because two would drift.** The proof and the stories used to be
// able to disagree about what an unanswered read comes back as, and a story
// that drew a read the proof never rendered would be the gap this package's
// fixtures exist to close.

import type {
  CallRead,
  CheckOutputRead,
  FrameRead,
  Followed,
  ModelChoices,
  Opened,
  Outcome,
} from "@armada/protocol";

import type { JobDetailProps } from "../JobDetail";
import type { JobFixture } from "./fixture";

const NOT_CONNECTED: Outcome = { ok: false, why: "not_connected" };
const UNKNOWN_JOB_OPENED: Opened = { ok: false, why: "unknown_job" };
const UNKNOWN_JOB_FOLLOWED: Followed = { ok: false, why: "unknown_job" };
const NOT_ANSWERED_CALL: CallRead = { ok: false, outcome: NOT_CONNECTED };
const NOT_ANSWERED_OUTPUT: CheckOutputRead = { ok: false, outcome: NOT_CONNECTED };
const NOT_ANSWERED_FRAME: FrameRead = { ok: false, outcome: NOT_CONNECTED };

/** What `list_models` answers on every fixture, in the spelling Fleet uses. */
const MODELS: ModelChoices = { models: ["haiku", "sonnet", "opus"], default: "sonnet" };

function noop(): void {}

/**
 * Every prop `JobDetail` needs, answered from one fixture's own reads and
 * no-ops beyond it. **Reads are answered; acts do nothing.** A story is a
 * reading of one moment, so a press that would change the Job has nowhere to
 * go — and the proof that renders every fixture never presses anything.
 */
export function propsFor(fixture: JobFixture): JobDetailProps {
  return {
    job: fixture.job,
    watched: fixture.watched,
    workflows: fixture.workflows,
    manifests: fixture.manifests,
    stale: false,
    now: fixture.now,
    acting: false,
    approving: false,
    deciding: false,
    onAct: noop,
    onRedirect: noop,
    onAnswer: noop,
    onAnswerCommand: noop,
    onAnswerJudge: noop,
    onSetWhenBlocked: noop,
    onSetWhenRefused: noop,
    onSetModel: noop,
    onRemoveAllowedCommand: noop,
    models: MODELS,
    onOverrule: noop,
    onRaiseCap: noop,
    onRaiseTurnCap: noop,
    onRerun: noop,
    onReadDiff: noop,
    onOpenArtifact: async () => UNKNOWN_JOB_OPENED,
    onOpenPullRequest: async () => UNKNOWN_JOB_FOLLOWED,
    onOpenRemarkLink: noop,
    onReadCall: async (_jobId, callId) => fixture.calls[callId] ?? NOT_ANSWERED_CALL,
    // Offered on every fixture so the control is drawn, and answered as a
    // refusal: a story is a reading of one moment and no Fleet is behind it.
    onExplainCommand: async () => ({ ok: false, outcome: NOT_CONNECTED }),
    onReadCheckOutput: async (_jobId, kept) => fixture.checkOutputs[kept] ?? NOT_ANSWERED_OUTPUT,
    onReadFrame: async (_jobId, kept) => fixture.frames[kept] ?? NOT_ANSWERED_FRAME,
    onNeedMaterial: noop,
    onNeedRemarks: noop,
    onReport: async () => NOT_CONNECTED,
    onApprove: noop,
    onMergePullRequest: noop,
    onResolvePullRequestConflict: noop,
    onApproveReview: noop,
    onRequestChanges: noop,
    onReject: noop,
    onTakeUpRemarks: noop,
    observed: fixture.observed,
    journalled: fixture.journalled,
    resources: fixture.resources,
    history: fixture.history,
    examination: { state: "none" },
    onExamine: noop,
    recorded: fixture.recorded,
    onCopied: noop,
    onSaid: noop,
    // Journey 9's run sheet. A story is a reading of one moment and nothing
    // here has opened the sheet, so the reads are `none` and every act is a
    // no-op, on this function's own rule.
    rehearsal: {
      runSheet: { state: "none" },
      runFollowed: { state: "none" },
      servers: { servers: [] },
      onWatchRunSheet: noop,
      onObserveRun: noop,
      onStartRun: async () => NOT_CONNECTED,
      onStopRun: async () => NOT_CONNECTED,
      onUndoRun: async () => NOT_CONNECTED,
      onListRuns: async () => ({ ok: false, outcome: NOT_CONNECTED }),
      onGetRunOutput: async () => ({ ok: false, outcome: NOT_CONNECTED }),
      onStartServer: async () => NOT_CONNECTED,
      onStopServer: async () => NOT_CONNECTED,
      onOpenServerLink: async () => UNKNOWN_JOB_FOLLOWED,
    },
  };
}
