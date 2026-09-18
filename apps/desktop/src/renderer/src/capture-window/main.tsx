import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";

import "../styles/capture.css";
import type { BridgeIdentity } from "@armada/protocol";
import { Boundary } from "@armada/shell";
import { NOTHING_YET } from "../../../shared/bridge";
import { CaptureWindow } from "./CaptureWindow";

// The capture window's bar — #1294. A second renderer entry, and the only one
// besides Bridge's own: it is a separate document because it is a separate
// view, composited over a page Bridge did not author.
//
// **The boundary is outside the bar**, `main.tsx`'s reason: what it exists to
// catch is the whole tree going, and a boundary inside cannot catch that. With
// it gone a person would be looking at another repository's app with no sign
// of what it is.

/** Who Bridge is, read once — the least state that can sit above the boundary. */
function Root() {
  const [bridge, setBridge] = useState<BridgeIdentity>(NOTHING_YET.bridge);

  useEffect(() => {
    void window.armada.state().then((state) => setBridge(state.bridge));
  }, []);

  return (
    <Boundary region="the capture bar" usable={false} bridge={bridge}>
      <CaptureWindow api={window.armada.captureWindow} />
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
