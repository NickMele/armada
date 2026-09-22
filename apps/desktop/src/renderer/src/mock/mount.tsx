// The app on a scenario, mounted — for the mock's own page and for a browser test.
//
// **`App` itself, under the root boundary `main.tsx` puts it under**, so what a
// test asserts against is the window Bridge draws. `main.tsx` is not imported
// because it mounts itself on import.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { Boundary } from "@armada/shell";
import { HapticsProvider } from "@armada/components";

import { BoardDrafts } from "@armada/screens/src/boards";

import "../styles/index.css";
import type { BridgeApi } from "../../../shared/api";
import { App } from "../App";
import { fakeBridge } from "./fake";
import { scenarioNamed } from "./scenario";
import type { Scenario } from "./scenario";

/** What was mounted: the fake the window is talking to, and how to take it down. */
export type Mounted = { api: BridgeApi; scenario: Scenario; unmount: () => void };

/**
 * Install a fake `window.armada` on `scenario` and mount the app into `host`.
 * **Throws on a name no scenario has**, so a test never passes against a default.
 */
export function mountApp(scenario: string | Scenario, host: HTMLElement, shared?: BridgeApi): Mounted {
  const chosen = typeof scenario === "string" ? scenarioNamed(scenario) : scenario;
  if (chosen === undefined) throw new Error(`no mock scenario named ${String(scenario)}`);
  // `shared` is a second window on the same main: both hear what either one's Fleet publishes.
  const api = shared ?? fakeBridge(chosen);
  window.armada = api;
  const root = createRoot(host);
  root.render(
    <StrictMode>
      <Boundary region="the window" usable={false} bridge={chosen.state.bridge}>
        {/* As `main.tsx` mounts it, so a test can read what a press asked the trackpad to play. */}
        <HapticsProvider perform={(pattern) => api.tap(pattern)}>
          {/* The half of a board's reading no operation answers — `boards.tsx`.
              A window on a real Fleet mounts none, and each board falls back. */}
          <BoardDrafts draft={chosen.draft}>
            <App />
          </BoardDrafts>
        </HapticsProvider>
      </Boundary>
    </StrictMode>,
  );
  return { api, scenario: chosen, unmount: () => root.unmount() };
}
