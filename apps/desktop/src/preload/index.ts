import { contextBridge, ipcRenderer } from "electron";

import { CHANNELS } from "../shared/bridge";
import type { BridgeApi, BridgeState, Summons } from "../shared/bridge";
import type {
  CallRead,
  CheckOutputRead,
  FrameRead,
  ClearOutcome,
  Draft,
  Outcome,
  Proposed,
  ReclaimOutcome,
} from "@armada/protocol";
import type { FileReport } from "@armada/protocol";
import type { Artifact, Followed, Opened } from "@armada/protocol";
import type { ProtocolVersion } from "@armada/protocol";
import type { CommandAnswer, WhenBlocked } from "@armada/protocol";
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
  stopProposal: (): Promise<Outcome> => ipcRenderer.invoke(CHANNELS.stopProposal),

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

  // The disk, never the record — every row this reaches stays on the board,
  // under `Cleared`. One entry taking every id rather than a loop of
  // `killJob`-shaped calls at the call site, because the loop belongs to
  // main: it is the process holding the board these ids came off of.
  clearTerminalJobs: (jobIds: readonly string[]): Promise<ReclaimOutcome> =>
    ipcRenderer.invoke(CHANNELS.clearTerminalJobs, jobIds),

  // Real deletion, and the only entry here where that is true — every other
  // act moves a Job further, and this removes the row. A separate entry from
  // the one above for the reason `crates/ipc/operations.toml` gives: one call
  // with two unrelated things to fail at is worse than two calls.
  forgetTerminalJobs: (jobIds: readonly string[]): Promise<ClearOutcome> =>
    ipcRenderer.invoke(CHANNELS.forgetTerminalJobs, jobIds),

  // The per-Job half of `clearTerminalJobs` above. One id at a time — a
  // person reclaims the Job they are looking at, and the bulk shape exists
  // because clearing a board is a set.
  reclaimWorktree: (jobId: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.reclaimWorktree, jobId),

  // The per-Job half of `forgetTerminalJobs` above. Real deletion, and there
  // is no undo.
  forgetJob: (jobId: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.forgetJob, jobId),

  redirectDrone: (jobId: string, instruction: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.redirectDrone, jobId, instruction),
  answerQuestion: (
    jobId: string,
    questionId: string,
    chose: string,
  ): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.answerQuestion, jobId, questionId, chose),
  // A closed set on both, like the question's answer above: the answer is one
  // of the three Fleet offered for this call, and the setting one of two.
  answerCommand: (jobId: string, call: string, answer: CommandAnswer): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.answerCommand, jobId, call, answer),
  setWhenBlocked: (jobId: string, whenBlocked: WhenBlocked): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.setWhenBlocked, jobId, whenBlocked),
  // The other two settings on one job. `null` crosses as `null`, which is the
  // clear Fleet asks for by name — not `undefined`, which it would refuse.
  setModel: (jobId: string, model: string | null): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.setModel, jobId, model),
  removeAllowedCommand: (jobId: string, run: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.removeAllowedCommand, jobId, run),

  // The note is optional, and `undefined` crosses as `undefined` — a plain
  // restart sends Fleet no body at all, which is the request it took before it
  // could read one.
  restartStep: (jobId: string, note?: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.restartStep, jobId, note),

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

  // Ask a Job to show its work. Its own entry because it moves nothing on the
  // Job: what comes back is a set of frames, or why there is none.
  showAgain: (jobId: string): Promise<Outcome> => ipcRenderer.invoke(CHANNELS.showAgain, jobId),

  // Give one job a higher cost ceiling. **Its own entry and never a general
  // update**: nothing else here sets a value on a job, and a capability that
  // could would be one press meaning whatever field it was handed. The figure
  // is millionths of a dollar, the unit `JobSpend` reads in, so the number
  // shown and the number sent are the same integer.
  raiseCostCap: (jobId: string, costCapMicros: number): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.raiseCostCap, jobId, costCapMicros),

  // Let one job take more turns. Its own entry beside the cost cap rather than
  // a unit on it: the two clear different holds, and one capability taking
  // which ceiling to move would be a press whose effect depends on an argument.
  // The figure is a plain turn count, the unit `JobSpend` reads it in.
  raiseTurnCap: (jobId: string, turnCap: number): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.raiseTurnCap, jobId, turnCap),

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

  // One running Check's log, as it is written. Read-only like the entry above,
  // and its own because it is its own socket.
  followCheckOutput: (jobId: string | null, kept: string | null): Promise<void> =>
    ipcRenderer.invoke(CHANNELS.followCheckOutput, jobId, kept),

  // One Job's transition history. Read-only like the two above it, and a
  // separate entry because it is a separate operation: a history is not a field
  // on the detail, so it is asked for when a surface unfolds rather than paid
  // for on every Job opened.
  readHistory: (jobId: string | null): Promise<void> =>
    ipcRenderer.invoke(CHANNELS.readHistory, jobId),

  // What the open Job holds on this machine. Read-only, and its own entry
  // because it is its own operation: `watchJob` is re-read on every event
  // naming the Job and this walks a process table and a directory.
  readResources: (jobId: string | null): Promise<void> =>
    ipcRenderer.invoke(CHANNELS.readResources, jobId),

  // Ask Fleet to go and look now. **The rung below intervene**, and the one
  // entry here that is an act and changes nothing: what it leaves is a line in
  // the Job's own log. It costs no model call, and the answer arrives on the
  // published state rather than coming back from the call.
  examineJob: (jobId: string): Promise<void> =>
    ipcRenderer.invoke(CHANNELS.examineJob, jobId),

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

  // What people wrote on a Job's open pull request. **The one read here that
  // reaches a forge**, so it is its own entry beside the two above rather than
  // riding one of them: a surface wanting the claims must not spend a process
  // and a network to get them, and nothing takes this on a timer.
  readRemarks: (jobId: string | null): Promise<void> =>
    ipcRenderer.invoke(CHANNELS.readRemarks, jobId),

  // The rest of one cut call argument. **A separate entry from `observeJob`,
  // and the narrowest read here**: it names a call id off a row this window was
  // already streamed rather than opening anything, and it answers once instead
  // of holding a subscription. Read-only like every entry around it — an
  // argument the record kept is a fact, and nothing on this channel can put one
  // there or reach the Drone that sent it.
  readCall: (jobId: string, callId: string): Promise<CallRead> =>
    ipcRenderer.invoke(CHANNELS.readCall, jobId, callId),
  readCheckOutput: (jobId: string, kept: string): Promise<CheckOutputRead> =>
    ipcRenderer.invoke(CHANNELS.readCheckOutput, jobId, kept),
  readFrame: (jobId: string, kept: string): Promise<FrameRead> =>
    ipcRenderer.invoke(CHANNELS.readFrame, jobId, kept),

  // Every report filed, with the counts. Read-only, and **the one read here
  // that carries no Job id**: a report survives the Job being forgotten, so
  // scoping the listing to a Job would lose the ones most worth reading. A
  // boolean, because there is nothing to scope it to — only whether a surface
  // is open. Nothing here files or withdraws a report; `fileReport` does the
  // first and nothing does the second.
  readReports: (want: boolean): Promise<void> =>
    ipcRenderer.invoke(CHANNELS.readReports, want),

  // What fleet is holding disk for, and the test each one did not pass. The
  // second read here with no job id, and for a different reason: what is being
  // decided is which of a set to give back, which no per-job field could ask.
  // Read-only — `reclaimWorktree` is the act, one job at a time, and it already
  // exists above. A piloted job's checkout is not in the answer at all: fleet
  // drops it, so nothing here can offer a directory a person is standing in.
  readHeld: (want: boolean): Promise<void> =>
    ipcRenderer.invoke(CHANNELS.readHeld, want),

  // The three decisions on the work, and they are three entries for the reason
  // the two kills are two: one capability taking "which decision" as an
  // argument would read as one act and perform three, and these three differ by
  // whether anything survives them. Approving takes the work, requesting
  // changes sends the drone back to the same step, and rejecting ends both.
  approveReview: (jobId: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.approveReview, jobId),

  mergePullRequest: (jobId: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.mergePullRequest, jobId),

  resolvePullRequestConflict: (jobId: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.resolvePullRequestConflict, jobId),

  requestChanges: (jobId: string, note: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.requestChanges, jobId, note),

  rejectWork: (jobId: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.rejectWork, jobId),

  // The fifth act at the same gate, and a fifth entry for the reason the four
  // above are four. **Handles, never words**: what crosses is what the forge
  // calls each comment, and fleet reads the pull request again to find out what
  // they say — so nothing on this side decides what a drone is told.
  takeUpRemarks: (jobId: string, remarks: string[]): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.takeUpRemarks, jobId, remarks),

  // The one entry that reaches outside the app, and the narrowest it can be:
  // a Job id and one of three words. **No path crosses here.** Main derives
  // the path from the Job and the repository its Manifest was read from, so
  // the renderer never holds the argument that decides what is opened — which
  // is what keeps this from being an arbitrary-file capability under a
  // friendly name. It is one entry rather than three because the three differ
  // only in which file and none of them changes anything about the Job.
  openArtifact: (jobId: string, what: Artifact): Promise<Opened> =>
    ipcRenderer.invoke(CHANNELS.openArtifact, jobId, what),

  // The second entry that reaches outside the app, and the only one that
  // leaves the machine. **A Job id and nothing else** — narrower than the one
  // above it, which at least takes a word. Main reads the address off the
  // reading it published and refuses anything that is not `https:`, so no
  // string the renderer composed can reach `shell.openExternal`; the renderer
  // draws the address, and does not send it.
  openPullRequest: (jobId: string): Promise<Followed> =>
    ipcRenderer.invoke(CHANNELS.openPullRequest, jobId),

  // One comment's own link, `openPullRequest`'s reason exactly: a Job id and a
  // comment id, never an address the renderer composed.
  openRemarkLink: (jobId: string, remarkId: string): Promise<Followed> =>
    ipcRenderer.invoke(CHANNELS.openRemarkLink, jobId, remarkId),

  // Where a pressed notification says to go. **The only entry here that the
  // renderer does not initiate** — every other one is the window asking, and
  // this is main handing over a press that happened outside it, possibly with
  // no window on screen at the time.
  //
  // `subscribe`'s shape, and for its reason: a listener plus the function that
  // removes it, so a component that mounts twice does not leave one behind. It
  // reaches nothing and grants nothing: what arrives is a Job id this window
  // already draws, or `null` for the set.
  onSummoned: (onGo: (to: Summons) => void): (() => void) => {
    const handler = (_event: unknown, to: Summons): void => onGo(to);
    ipcRenderer.on(CHANNELS.summoned, handler);
    return () => {
      ipcRenderer.removeListener(CHANNELS.summoned, handler);
    };
  },
};

contextBridge.exposeInMainWorld("armada", api);
