import { contextBridge, ipcRenderer } from "electron";

import { CHANNELS } from "../shared/bridge";
import { ANNOTATE_FLAG, ANNOTATION_CHANNELS } from "../shared/annotations";
import type { Annotation, AnnotationsDevApi, Box } from "../shared/annotations";
import { frameStreamUrl } from "../shared/streaming";
import type { BridgeState, Summons } from "../shared/bridge";
import type { BridgeApi, CommandExplainedRead } from "../shared/api";
import type { Pattern } from "../shared/haptics";
import type {
  CallRead,
  CheckOutputRead,
  FrameRead,
  ClearOutcome,
  Draft,
  Outcome,
  Proposed,
  ReclaimOutcome,
  StagedAttachment,
} from "@armada/protocol";
import type { FileReport } from "@armada/protocol";
import type { HelmContext } from "@armada/protocol";
import type { StudioCapture, StudioNodeByHand, StudioPosition, StudioPromotion } from "@armada/protocol";
import type { StudioAnswer } from "@armada/screens/src/studio-reads";
import type { AddTask, DropTask } from "@armada/protocol";
import type { PlanEditAnswer } from "@armada/screens/src/plan-edits";
import type { Artifact, Followed, Opened } from "@armada/protocol";
import type { ProtocolVersion, RunListRead, RunOutputRead, StartRun } from "@armada/protocol";
import type { CheckoutRunListRead, StartCheckoutRun } from "@armada/protocol";
import type { EditManifest, SaveManifestFile } from "@armada/protocol";
import type {
  ManifestEditAnswer,
  ManifestFileRead,
  ManifestSaveAnswer,
  ManifestSpendRead,
} from "@armada/screens/src/editing";
import type { CheckoutRunDiffRead } from "@armada/protocol";
import type { RepositoryAllowedCommandsRead } from "@armada/screens/src/manifest-allows";
import type { LocateAnswer } from "@armada/screens/src/locate-reads";
import type { ComposingRead } from "@armada/screens/src/composing-reads";
import type { EditManifestProposal, WriteManifestProposal } from "@armada/protocol";
import type {
  ManifestProposalsRead,
  ProposalAnswer,
  RepositoryScanRead,
} from "@armada/screens/src/setup-reads";
import type {
  CommandAnswer,
  HelmCallAnswer,
  JudgeAnswer,
  SaveLimits,
  SavePreference,
  WhenBlocked,
  WhenRefused,
} from "@armada/protocol";
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
  //
  // `repository` is the root New job's ask answered on All, so the request
  // names it rather than the pick — #959. `null` where a repository was
  // already picked, as it always was.
  proposeFromRequest: (
    request: string,
    attachments: readonly StagedAttachment[],
    repository: string | null = null,
  ): Promise<Proposed> =>
    ipcRenderer.invoke(CHANNELS.proposeFromRequest, request, attachments, repository),
  stopProposal: (): Promise<Outcome> => ipcRenderer.invoke(CHANNELS.stopProposal),

  // Bytes never round-trip through `proposeJob`'s JSON channel as base64 —
  // this writes them to a staging file and hands back the path a later
  // `proposeJob` call carries as a `staged_path`.
  stageAttachment: (bytes: ArrayBuffer, filename: string, mimeType: string): Promise<{ path: string }> =>
    ipcRenderer.invoke(CHANNELS.stageAttachment, bytes, filename, mimeType),

  searchFiles: (query: string): Promise<string[]> =>
    ipcRenderer.invoke(CHANNELS.searchFiles, query),

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

  // A force, unlike the reclaim above — Fleet's 409 is the safety net a
  // stale confirmation needs.
  deleteBranch: (jobId: string, tip: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.deleteBranch, jobId, tip),

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
  // The answer is one of the three Fleet offered for this call. The note is a
  // person's own words, read only on a reject, and `undefined` crosses as
  // `undefined` — a bare refusal sends no prose at all. The rule is one of
  // that call's own candidates, read only on an always-allow.
  answerCommand: (
    jobId: string,
    call: string,
    answer: CommandAnswer,
    note?: string,
    rule?: string,
  ): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.answerCommand, jobId, call, answer, note, rule),
  // One held helm call. The note is a person's own words, read only on a
  // refusal; `undefined` crosses as `undefined`.
  answerHelmCall: (call: string, answer: HelmCallAnswer, note?: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.answerHelmCall, call, answer, note),
  // What that command does. A read, and the one capability here that answers a
  // question about a call rather than deciding it.
  explainCommand: (jobId: string, callId: string): Promise<CommandExplainedRead> =>
    ipcRenderer.invoke(CHANNELS.explainCommand, jobId, callId),
  setWhenBlocked: (jobId: string, whenBlocked: WhenBlocked): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.setWhenBlocked, jobId, whenBlocked),
  answerJudge: (jobId: string, askedAt: string, answer: JudgeAnswer, note?: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.answerJudge, jobId, askedAt, answer, note),
  setWhenRefused: (jobId: string, whenRefused: WhenRefused): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.setWhenRefused, jobId, whenRefused),
  // The other two settings on one job. `null` crosses as `null`, which is the
  // clear Fleet asks for by name — not `undefined`, which it would refuse.
  setModel: (jobId: string, model: string | null): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.setModel, jobId, model),
  setReviewModel: (jobId: string, model: string | null): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.setReviewModel, jobId, model),
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

  // Run a stopped step's Checks again. Its own entry beside `rerunGate`'s, for
  // the trigger the two partition on — `#1105`.
  rerunChecks: (jobId: string): Promise<Outcome> => ipcRenderer.invoke(CHANNELS.rerunChecks, jobId),

  // Ask a Job to show its work. Its own entry because it moves nothing on the
  // Job: what comes back is a set of frames, or why there is none. `spec` names
  // which to run; without one Fleet runs the last a Drone named.
  showAgain: (jobId: string, spec?: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.showAgain, jobId, spec),

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

  // Fleet's three admission limits. **Fleet-wide, and no Job id crosses this
  // channel** — the only act on this surface that names none.
  saveLimits: (values: SaveLimits): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.saveLimits, values),

  // A person's Bridge preferences. **Fleet-wide**, `saveLimits`' reason.
  savePreference: (save: SavePreference): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.savePreference, save),

  // Say a job failed in error, and file its record with the reason. **Its own
  // entry and not a mode on `overrideVerdict`**: that one moves the job past a
  // verdict, and this one moves nothing at all — one capability doing both
  // would make "I think this was wrong" and "let it through anyway" the same
  // press. Nothing here reaches the issue tracker; what comes back is a record.
  fileReport: (jobId: string, filing: FileReport): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.fileReport, jobId, filing),

  // A person's own add or drop of a task on the Job's plan — `#897`. Each
  // answers with the plan the change leaves, not a plain `Outcome`.
  addTask: (jobId: string, add: AddTask): Promise<PlanEditAnswer> =>
    ipcRenderer.invoke(CHANNELS.addTask, jobId, add),
  dropTask: (jobId: string, drop: DropTask): Promise<PlanEditAnswer> =>
    ipcRenderer.invoke(CHANNELS.dropTask, jobId, drop),

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

  // The run sheet — Journey 9. Opened by the sheet rather than by the Job,
  // `readDiff`'s reason: the Manifest a Job froze is read on the press that
  // draws it, not on every Job opened.
  watchRunSheet: (jobId: string | null): Promise<void> =>
    ipcRenderer.invoke(CHANNELS.watchRunSheet, jobId),

  // One run's output, read-only like `followCheckOutput` beside it and its
  // own socket for the same reason.
  observeRun: (jobId: string | null, runId: string | null): Promise<void> =>
    ipcRenderer.invoke(CHANNELS.observeRun, jobId, runId),

  // Run one Check or Command in this Job's worktree. **A rehearsal**: no
  // Evidence, no `job_step_checks` row, nothing on the Job moves.
  startRun: (jobId: string, body: StartRun): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.startRun, jobId, body),

  stopRun: (jobId: string, runId: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.stopRun, jobId, runId),

  undoRun: (jobId: string, runId: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.undoRun, jobId, runId),

  listRuns: (jobId: string): Promise<RunListRead> => ipcRenderer.invoke(CHANNELS.listRuns, jobId),

  getRunOutput: (jobId: string, runId: string): Promise<RunOutputRead> =>
    ipcRenderer.invoke(CHANNELS.getRunOutput, jobId, runId),

  // The same rehearsal in the main checkout — the Manifest surface. **Its own
  // entries rather than a `jobId` that may be `null` on the seven above**: a
  // route under `/manifest` and a route under `/jobs/:id` are two operations,
  // and one capability taking which would read as one act and perform two.
  watchCheckoutRunSheet: (want: boolean): Promise<void> =>
    ipcRenderer.invoke(CHANNELS.watchCheckoutRunSheet, want),

  observeCheckoutRun: (runId: string | null): Promise<void> =>
    ipcRenderer.invoke(CHANNELS.observeCheckoutRun, runId),

  // A name and nothing else. There is no frozen Manifest to choose against
  // and no diff to narrow to in the main checkout.
  startCheckoutRun: (body: StartCheckoutRun): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.startCheckoutRun, body),

  stopCheckoutRun: (runId: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.stopCheckoutRun, runId),

  undoCheckoutRun: (runId: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.undoCheckoutRun, runId),

  listCheckoutRuns: (): Promise<CheckoutRunListRead> =>
    ipcRenderer.invoke(CHANNELS.listCheckoutRuns),

  getCheckoutRunOutput: (runId: string): Promise<RunOutputRead> =>
    ipcRenderer.invoke(CHANNELS.getCheckoutRunOutput, runId),

  // A read, and one run's only: main composes the route, and the id is
  // checked against the checkout's own runs by Fleet before any tree is read.
  getCheckoutRunDiff: (runId: string): Promise<CheckoutRunDiffRead> =>
    ipcRenderer.invoke(CHANNELS.getCheckoutRunDiff, runId),

  // Drift, held open by the Manifest surface; Verify, pressed on it.
  watchManifestDrift: (want: boolean): Promise<void> =>
    ipcRenderer.invoke(CHANNELS.watchManifestDrift, want),

  // Overview's two reads, held open by that surface. A boolean: main names every route.
  watchOverview: (want: boolean): Promise<void> => ipcRenderer.invoke(CHANNELS.watchOverview, want),

  startCheckoutVerify: (workspace?: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.startCheckoutVerify, workspace),

  // The Manifest file. **A write into the repository**, and still no path: the
  // renderer hands over text and what it started from, and Fleet decides
  // where it lands and whether the disk still matches.
  readManifestFile: (): Promise<ManifestFileRead> => ipcRenderer.invoke(CHANNELS.readManifestFile),

  saveManifestFile: (body: SaveManifestFile): Promise<ManifestSaveAnswer> =>
    ipcRenderer.invoke(CHANNELS.saveManifestFile, body),
  // Edits by key, and past Jobs' spend. Still no path: Fleet names the file.
  editManifest: (body: EditManifest): Promise<ManifestEditAnswer> =>
    ipcRenderer.invoke(CHANNELS.editManifest, body),
  readManifestSpend: (): Promise<ManifestSpendRead> => ipcRenderer.invoke(CHANNELS.readManifestSpend),
  // Setup: Scan, the proposals, one edit, Write — one operation each.
  readRepositoryScan: (): Promise<RepositoryScanRead> => ipcRenderer.invoke(CHANNELS.readRepositoryScan),
  readManifestProposals: (): Promise<ManifestProposalsRead> =>
    ipcRenderer.invoke(CHANNELS.readManifestProposals),
  editManifestProposal: (body: EditManifestProposal): Promise<ProposalAnswer> =>
    ipcRenderer.invoke(CHANNELS.editManifestProposal, body),
  writeManifestProposal: (body: WriteManifestProposal): Promise<ProposalAnswer> =>
    ipcRenderer.invoke(CHANNELS.writeManifestProposal, body),

  // A repository-wide always-allow — Fleet's own table since protocol 13.5.
  // Neither takes a path or a job id: Fleet names the repository.
  listRepositoryAllowedCommands: (): Promise<RepositoryAllowedCommandsRead> =>
    ipcRenderer.invoke(CHANNELS.listRepositoryAllowedCommands),

  removeRepositoryAllowedCommand: (run: string): Promise<RepositoryAllowedCommandsRead> =>
    ipcRenderer.invoke(CHANNELS.removeRepositoryAllowedCommand, run),

  // Start a declared server, for this Job's worktree or, with no Job, the
  // main checkout — the capability the Manifest surface shares, which is why
  // `jobId` was optional here before that surface existed.
  pickRepository: (root: string | null): Promise<void> => ipcRenderer.invoke(CHANNELS.pickRepository, root),

  // Locate: the OS folder dialog, a clone parent as Fleet will spell it, and a repository added or cloned.
  chooseFolder: (): Promise<string | null> => ipcRenderer.invoke(CHANNELS.chooseFolder),
  resolveFolder: (path: string): Promise<string | null> => ipcRenderer.invoke(CHANNELS.resolveFolder, path),
  addRepository: (path: string): Promise<LocateAnswer> => ipcRenderer.invoke(CHANNELS.addRepository, path),
  cloneRepository: (url: string, parent: string): Promise<LocateAnswer> => ipcRenderer.invoke(CHANNELS.cloneRepository, url, parent),

  startServer: (name: string, jobId?: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.startServer, name, jobId),

  stopServer: (serverId: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.stopServer, serverId),

  // The third entry that reaches outside the app. A server id and the link's
  // own address, `openRemarkLink`'s reason: main checks the address against
  // what it is already holding for that server before it opens anything, so
  // no string a click handler composed reaches `shell.openExternal` unchecked.
  openServerLink: (serverId: string, url: string): Promise<Followed> =>
    ipcRenderer.invoke(CHANNELS.openServerLink, serverId, url),

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
  // that spends the patch bytes**, and asked for more than once: on opening a
  // Job, on the press that opens the diff, and while that sheet is open and
  // the file list moves under it. `shared/api.ts` holds why.
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
  // New job's own reads on All — #959: `leftOut` and the Manifest reading for
  // the repository the ask answered, named by root since the pick stays put.
  // `readCall`'s shape: answered once, and nothing here is held or republished.
  readComposing: (repository: string): Promise<ComposingRead> =>
    ipcRenderer.invoke(CHANNELS.readComposing, repository),

  // A recording's address, composed rather than fetched — it is the one entry
  // here that opens no channel. What makes it safe to hand over is what makes
  // the rest safe: it names a Job and a frame id the record already gave out,
  // and main is what turns that into a request.
  frameStreamUrl: (jobId: string, kept: string): string => frameStreamUrl(jobId, kept),

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
  // Read-only — `reclaimWorktree`, `deleteBranch` and `forgetJob` are the
  // three acts, one job at a time, and they already exist above. A piloted
  // job's checkout is not in the answer at all: fleet drops it, so nothing
  // here can offer a directory a person is standing in.
  readHeld: (want: boolean): Promise<void> =>
    ipcRenderer.invoke(CHANNELS.readHeld, want),

  // A repository's Studios, and one open — #1287. Two reads a surface holds, four acts, each one
  // operation with its own ids: nothing here names a route or reaches a node on another Studio.
  watchStudios: (manifestId: string | null): Promise<void> =>
    ipcRenderer.invoke(CHANNELS.watchStudios, manifestId),
  watchStudio: (studioId: string | null): Promise<void> => ipcRenderer.invoke(CHANNELS.watchStudio, studioId),
  createStudio: (manifestId: string): Promise<StudioAnswer> =>
    ipcRenderer.invoke(CHANNELS.createStudio, manifestId),
  renameStudio: (studioId: string, name: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.renameStudio, studioId, name),
  // Three kinds and no more: the type is as narrow as the act, so the surface
  // this bridge gains is a note, a link or a sketch rather than any node.
  addStudioNode: (studioId: string, node: StudioNodeByHand, position: StudioPosition): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.addStudioNode, studioId, node, position),
  moveStudioNode: (studioId: string, nodeId: string, position: StudioPosition): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.moveStudioNode, studioId, nodeId, position),
  // The frame is main's: this hands over what was pointed at and nothing else,
  // so the capability added here is a Note on a Studio and not a screenshot.
  captureStudioNote: (studioId: string, said: string, capture: StudioCapture): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.captureStudioNote, studioId, said, capture),
  // Reading one, where the capture above takes one: the bytes come back to be
  // drawn and nothing about the file's place on disk crosses with them.
  readStudioFrame: (studioId: string, nodeId: string): Promise<FrameRead> =>
    ipcRenderer.invoke(CHANNELS.readStudioFrame, studioId, nodeId),
  removeStudioNode: (studioId: string, nodeId: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.removeStudioNode, studioId, nodeId),
  removeStudioNodes: (studioId: string, nodeIds: readonly string[]): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.removeStudioNodes, studioId, [...nodeIds]),
  decideStudioEdge: (studioId: string, edgeId: string, accepted: boolean): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.decideStudioEdge, studioId, edgeId, accepted),
  promoteOnStudio: (studioId: string, promotion: StudioPromotion): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.promoteOnStudio, studioId, promotion),

  // The three decisions on the work, and they are three entries for the reason
  // the two kills are two: one capability taking "which decision" as an
  // argument would read as one act and perform three, and these three differ by
  // whether anything survives them. Approving takes the work, requesting
  // changes sends the drone back to the same step, and rejecting ends both.
  approveReview: (jobId: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.approveReview, jobId),

  mergePullRequest: (jobId: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.mergePullRequest, jobId),

  rerunFailedChecks: (jobId: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.rerunFailedChecks, jobId),

  investigateFailedChecks: (jobId: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.investigateFailedChecks, jobId),

  queueAfterFinding: (jobId: string, finding: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.queueAfterFinding, jobId, finding),

  fileFindingIssue: (jobId: string, finding: string, title: string, body: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.fileFindingIssue, jobId, finding, title, body),

  // A finding's issue, `openRemarkLink`'s reason: main reads the address off the Job it holds.
  openFindingIssue: (jobId: string, finding: string): Promise<Followed> =>
    ipcRenderer.invoke(CHANNELS.openFindingIssue, jobId, finding),

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

  dismissFinding: (jobId: string, finding: string, reason: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.dismissFinding, jobId, finding, reason),

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

  // What a Studio node points at — #1406. `openPullRequest`'s reason: a Studio
  // id and a node id, never an address the renderer composed.
  openStudioNode: (studioId: string, nodeId: string): Promise<Followed> =>
    ipcRenderer.invoke(CHANNELS.openStudioNode, studioId, nodeId),

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

  askHelm: (text: string, context?: HelmContext): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.askHelm, text, context),
  startHelmFresh: (): Promise<Outcome> => ipcRenderer.invoke(CHANNELS.startHelmFresh),
  pointHelm: (manifestId: string): Promise<void> => ipcRenderer.invoke(CHANNELS.pointHelm, manifestId),

  tap: (pattern: Pattern): void => ipcRenderer.send(CHANNELS.tap, pattern),
};

contextBridge.exposeInMainWorld("armada", api);

// The dev-only annotation layer, #1226 — never on `armada`, which is Fleet's seam.
// Main passes the flag only when the app is not packaged.
if (process.argv.includes(ANNOTATE_FLAG)) {
  const annotations: AnnotationsDevApi = {
    list: (): Promise<Annotation[]> => ipcRenderer.invoke(ANNOTATION_CHANNELS.list),
    save: (note: Annotation): Promise<void> => ipcRenderer.invoke(ANNOTATION_CHANNELS.save, note),
    remove: (id: string): Promise<void> => ipcRenderer.invoke(ANNOTATION_CHANNELS.remove, id),
    root: (): Promise<string | null> => ipcRenderer.invoke(ANNOTATION_CHANNELS.root),
    capture: (box: Box): Promise<ArrayBuffer | null> => ipcRenderer.invoke(ANNOTATION_CHANNELS.capture, box),
  };
  contextBridge.exposeInMainWorld("armadaDev", annotations);
}
