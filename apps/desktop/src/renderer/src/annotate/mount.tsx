// A root of its own, beside the app's rather than inside it: the app's error
// boundary, its keyboard handlers and its fibers never see the layer.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import { Layer } from "./Layer";
import { devServerSink, mainSink } from "./sink";

export function mount(): void {
  const host = document.createElement("div");
  host.setAttribute("data-armada-annotate", "");
  document.body.append(host);
  const sink = window.armadaDev !== undefined ? mainSink(window.armadaDev) : devServerSink();
  createRoot(host).render(
    <StrictMode>
      <Layer sink={sink} />
    </StrictMode>,
  );
}
