import type { BridgeApi } from "../../shared/bridge";

// What the preload put on the window, and the whole of what the renderer can
// reach. Declared rather than imported at runtime: the preload is a wire, not
// an import path.
declare global {
  interface Window {
    armada: BridgeApi;
  }
}

export {};
