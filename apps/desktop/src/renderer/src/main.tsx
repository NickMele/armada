import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";

import "./styles/index.css";
import type { BridgeIdentity } from "../../shared/bridge";
import { App, WAITING } from "./App";
import { Boundary } from "./Boundary";

// Bridge's renderer entry point. No Node, no `require`, no socket — everything
// it draws arrives through the preload from the one connection in the main
// process.
//
// **The root boundary is outside `App` on purpose.** The failure that started
// this was a title bar over an empty window, which means the whole tree went —
// a boundary inside `App` cannot catch what `App` itself throws. `Root` holds
// nothing but the path Bridge's log is at, so the fallback can still name it
// when everything under it has gone.

/**
 * Who Bridge is, read once. The only state above the boundary, and the least
 * that can be: everything this component can throw on, the fallback needs.
 */
function Root() {
  const [bridge, setBridge] = useState<BridgeIdentity>(WAITING.bridge);

  useEffect(() => {
    void window.armada.state().then((state) => setBridge(state.bridge));
  }, []);

  return (
    <Boundary
      region="the window"
      usable={false}
      bridge={bridge}
    >
      <App />
    </Boundary>
  );
}

const root = document.getElementById("root");
if (root !== null) {
  createRoot(root).render(
    <StrictMode>
      <Root />
    </StrictMode>,
  );
}
