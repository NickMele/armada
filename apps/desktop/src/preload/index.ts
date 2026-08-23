import { contextBridge } from 'electron'

// The whole surface the renderer is allowed to see. It is deliberately almost
// empty: every capability added here is one the renderer can reach, so the
// list is meant to stay short enough to read in one go.
contextBridge.exposeInMainWorld('armada', {
  protocolVersion: (): number => 1,
})
