import { contextBridge, ipcRenderer } from "electron";

import { CHANNELS } from "../shared/bridge";
import type { BridgeApi, BridgeState } from "../shared/bridge";
import type { CallRead, ClearOutcome, Draft, Outcome, Proposed } from "@armada/protocol";
import type { FileReport } from "@armada/protocol";
import type { Artifact, Opened } from "@armada/protocol";
import type { ProtocolVersion } from "@armada/protocol";
import { PROTOCOL_VERSION } from "@armada/protocol";

// The whole surface the renderer is allowed to see.
//
// **A preload surface is not the API the UI is meant to use, it is the API the
// UI is physically capable of using.** So every entry is one operation with a
// typed return — no raw `ipcRenderer`, no `require`, no filesystem handle, and
// no arbitrary-channel `invoke`, which would hand the renderer the whole main
// process under a thin name.
//
// The two kills are two entries on purpose. One capability taking "which kill"
// as an argument would be a surface that reads as one act and performs two.
//
// The protocol version is no longer a literal here: it is generated from
// `protocol-version.toml`, which both sides read.
const api: BridgeApi = {
  protocolVersion: (): ProtocolVersion => PROTOCOL_VERSION,

  state: (): Promise<BridgeState> => ipcRenderer.invoke(CHANNELS.state),

  subscribe: (onState: (state: BridgeState) => void): (() => void) => {
    const handler = (_event: unknown, state: BridgeState): void => onState(state);
    ipcRenderer.on(CHANNELS.changed, handler);
    return () => {
      ipcRenderer.removeListener(CHANNELS.changed, handler);
    };
  },

  proposeJob: (draft: Draft): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.proposeJob, draft),

  // Describing the work instead of naming a workflow. Its own entry beside
  // `proposeJob` rather than a mode on it: one carries a workflow the person
  // chose and the other carries the sentence they wrote, and a single
  // capability taking which would read as one act and perform two.
  proposeFromRequest: (request: string): Promise<Proposed> =>
    ipcRenderer.invoke(CHANNELS.proposeFromRequest, request),

  // Bytes never round-trip through `proposeJob`'s JSON channel as base64 —
  // this writes them to a staging file and hands back the path a later
  // `proposeJob` call carries as a `staged_path`.
  stageAttachment: (bytes: ArrayBuffer, filename: string, mimeType: string): Promise<{ path: string }> =>
    ipcRenderer.invoke(CHANNELS.stageAttachment, bytes, filename, mimeType),

  approveDispatch: (jobId: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.approveDispatch, jobId),

  redispatchJob: (jobId: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.redispatchJob, jobId),

  killDrone: (jobId: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.killDrone, jobId),

  killJob: (jobId: string): Promise<Outcome> => ipcRenderer.invoke(CHANNELS.killJob, jobId),

  // Real deletion, and the only entry here where that is true — every other
  // act moves a Job further, and this removes the row. One entry taking every
  // id rather than a loop of `killJob`-shaped calls at the call site, because
  // the loop belongs to main: it is the process holding the board these ids
  // came off of.
  clearTerminalJobs: (jobIds: readonly string[]): Promise<ClearOutcome> =>
    ipcRenderer.invoke(CHANNELS.clearTerminalJobs, jobIds),

  redirectDrone: (jobId: string, instruction: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.redirectDrone, jobId, instruction),
  answerQuestion: (
    jobId: string,
    questionId: string,
    chose: string,
  ): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.answerQuestion, jobId, questionId, chose),

  restartStep: (jobId: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.restartStep, jobId),

  // Overrule a Judge that refused the work. **Its own entry and never a flag on
  // `approveReview`** — that one answers a gate nothing objected to, this one
  // answers a gate that refused, and one capability taking which would let a
  // refusal be taken with the act built for work nobody argued about. The
  // reason is the second argument because it is required: Fleet answers 422
  // without one, and an override that says nothing is the thing the act is
  // refused for.
  overrideVerdict: (jobId: string, reason: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.overrideVerdict, jobId, reason),

  // Ask the gate again where it could not decide. **Its own entry beside the
  // override rather than a flag on it**: that one lifts a decision a machine
  // made, and this one asks for a decision no machine reached. Fleet's two
  // routes refuse each other's triggers, so one capability taking which would
  // be a surface that reads as one act and performs two acts that partition. No
  // reason crosses, because nothing is being disagreed with.
  rerunGate: (jobId: string): Promise<Outcome> => ipcRenderer.invoke(CHANNELS.rerunGate, jobId),

  // Say a job failed in error, and file its record with the reason. **Its own
  // entry and not a mode on `overrideVerdict`**: that one moves the job past a
  // verdict, and this one moves nothing at all — one capability doing both
  // would make "I think this was wrong" and "let it through anyway" the same
  // press. Nothing here reaches the issue tracker; what comes back is a record.
  fileReport: (jobId: string, filing: FileReport): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.fileReport, jobId, filing),

  watchJob: (jobId: string | null): Promise<void> =>
    ipcRenderer.invoke(CHANNELS.watchJob, jobId),

  // Read-only, and it is a separate entry from `watchJob` because it is a
  // separate act: one reads a Job's record, the other watches its Drone work.
  // Neither can send anything to a Drone, and no entry here ever will.
  observeJob: (jobId: string | null): Promise<void> =>
    ipcRenderer.invoke(CHANNELS.observeJob, jobId),

  // One Job's transition history. Read-only like the two above it, and a
  // separate entry because it is a separate operation: a history is not a field
  // on the detail, so it is asked for when a surface unfolds rather than paid
  // for on every Job opened.
  readHistory: (jobId: string | null): Promise<void> =>
    ipcRenderer.invoke(CHANNELS.readHistory, jobId),

  // What a Job's Drones claimed. Read-only, and a separate entry from the diff
  // below because they are two operations: this one is four lines per step and
  // that one is the patch, and a surface wanting only the claims must not have
  // to fetch a megabyte to read them.
  readEvidence: (jobId: string | null): Promise<void> =>
    ipcRenderer.invoke(CHANNELS.readEvidence, jobId),

  // The worktree against the branch it was cut from — **the one entry here
  // that spends the patch bytes**. Deliberately not reached by opening a Job:
  // the renderer calls it from the surface that shows a diff, which is the act
  // the bytes were separated for.
  readDiff: (jobId: string | null): Promise<void> =>
    ipcRenderer.invoke(CHANNELS.readDiff, jobId),

  // The rest of one cut call argument. **A separate entry from `observeJob`,
  // and the narrowest read here**: it names a call id off a row this window was
  // already streamed rather than opening anything, and it answers once instead
  // of holding a subscription. Read-only like every entry around it — an
  // argument the record kept is a fact, and nothing on this channel can put one
  // there or reach the Drone that sent it.
  readCall: (jobId: string, callId: string): Promise<CallRead> =>
    ipcRenderer.invoke(CHANNELS.readCall, jobId, callId),

  // Every report filed, with the counts. Read-only, and **the one read here
  // that carries no Job id**: a report survives the Job being forgotten, so
  // scoping the listing to a Job would lose the ones most worth reading. A
  // boolean, because there is nothing to scope it to — only whether a surface
  // is open. Nothing here files or withdraws a report; `fileReport` does the
  // first and nothing does the second.
  readReports: (want: boolean): Promise<void> =>
    ipcRenderer.invoke(CHANNELS.readReports, want),

  // The three decisions on the work, and they are three entries for the reason
  // the two kills are two: one capability taking "which decision" as an
  // argument would read as one act and perform three, and these three differ by
  // whether anything survives them. Approving takes the work, requesting
  // changes sends the drone back to the same step, and rejecting ends both.
  approveReview: (jobId: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.approveReview, jobId),

  requestChanges: (jobId: string, note: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.requestChanges, jobId, note),

  rejectWork: (jobId: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.rejectWork, jobId),

  // The one entry that reaches outside the app, and the narrowest it can be:
  // a Job id and one of three words. **No path crosses here.** Main derives
  // the path from the Job and the repository its Manifest was read from, so
  // the renderer never holds the argument that decides what is opened — which
  // is what keeps this from being an arbitrary-file capability under a
  // friendly name. It is one entry rather than three because the three differ
  // only in which file and none of them changes anything about the Job.
  openArtifact: (jobId: string, what: Artifact): Promise<Opened> =>
    ipcRenderer.invoke(CHANNELS.openArtifact, jobId, what),
};

contextBridge.exposeInMainWorld("armada", api);
