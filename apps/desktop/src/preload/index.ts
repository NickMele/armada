import { contextBridge, ipcRenderer } from "electron";

import { CHANNELS } from "../shared/bridge";
import type { BridgeApi, BridgeState, Draft, Outcome } from "../shared/bridge";
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
// `protocol-version.toml`, the one number both sides check.
const api: BridgeApi = {
  protocolVersion: (): number => PROTOCOL_VERSION,

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

  approveDispatch: (jobId: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.approveDispatch, jobId),

  redispatchJob: (jobId: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.redispatchJob, jobId),

  killDrone: (jobId: string): Promise<Outcome> =>
    ipcRenderer.invoke(CHANNELS.killDrone, jobId),

  killJob: (jobId: string): Promise<Outcome> => ipcRenderer.invoke(CHANNELS.killJob, jobId),

  watchJob: (jobId: string | null): Promise<void> =>
    ipcRenderer.invoke(CHANNELS.watchJob, jobId),

  // Read-only, and it is a separate entry from `watchJob` because it is a
  // separate act: one reads a Job's record, the other watches its Drone work.
  // Neither can send anything to a Drone, and no entry here ever will.
  observeJob: (jobId: string | null): Promise<void> =>
    ipcRenderer.invoke(CHANNELS.observeJob, jobId),
};

contextBridge.exposeInMainWorld("armada", api);
