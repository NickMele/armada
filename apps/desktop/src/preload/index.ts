import { contextBridge, ipcRenderer } from "electron";

import { CHANNELS } from "../shared/bridge";
import type { BridgeApi, BridgeState, Draft, Outcome } from "../shared/bridge";
import type { ProtocolVersion } from "../shared/version";
import { PROTOCOL_VERSION } from "../shared/generated/protocol-version";

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

  redirectDrone: (jobId: string, instruction: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.redirectDrone, jobId, instruction),

  restartStep: (jobId: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.restartStep, jobId),

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
};

contextBridge.exposeInMainWorld("armada", api);
