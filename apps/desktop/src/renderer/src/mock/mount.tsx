// The app on a scenario, mounted — for the mock's own page and for a browser test.
//
// **`App` itself, under the root boundary `main.tsx` puts it under**, so what a
// test asserts against is the window Bridge draws. `main.tsx` is not imported
// because it mounts itself on import.

import { StrictMode, useEffect } from "react";
import { createRoot } from "react-dom/client";
import { Boundary } from "@armada/shell";
import { GuideShapeProvider, HapticsProvider } from "@armada/components";
import type { GuideShape } from "@armada/components";

import "../styles/index.css";
import type { BridgeApi } from "../../../shared/api";
import { App } from "../App";
import { DraftedFrom } from "../drafted";
import { fakeBridge } from "./fake";
import { scenarioNamed } from "./scenario";
import type { Scenario } from "./scenario";

/** What was mounted: the fake the window is talking to, and how to take it down. */
export type Mounted = {
  api: BridgeApi;
  scenario: Scenario;
  /**
   * Resolved once React has drawn the window and run its effects — which is
   * when the app's own key bindings exist.
   *
   * **`render` schedules the work rather than doing it**, so the line after
   * `mountApp` runs against an empty host. A press with a locator waits that
   * out on its own; a keystroke has none and nothing retries it, so it is
   * simply lost. Measured 23 Sep 2026 on #1592: under load `⌘J` reached
   * `window` with `#root` still empty, and Helm's dock never came up.
   */
  onScreen: Promise<void>;
  unmount: () => void;
};

/**
 * Says the window is up. **A sibling after `App` rather than a hook inside
 * it**: effects run in tree order, so this one runs after everything the app
 * mounted, this window's bindings included.
 */
function OnScreen({ say }: { say: () => void }) {
  useEffect(() => say(), [say]);
  return null;
}

/**
 * Install a fake `window.armada` on `scenario` and mount the app into `host`.
 * **Throws on a name no scenario has**, so a test never passes against a default.
 *
 * **`guides` is the mock's own, from `?guides=`, and `App` never sees it.** It
 * is a shape to compare guides in, not a setting: `main.tsx` reads it beside
 * `?scenario=`, this provider carries it, and the window under `pnpm dev` mounts
 * no provider and reads the default. `packages/components/src/guide-shape.tsx`.
 */
export function mountApp(
  scenario: string | Scenario,
  host: HTMLElement,
  shared?: BridgeApi,
  guides: GuideShape = "prose",
): Mounted {
  const chosen = typeof scenario === "string" ? scenarioNamed(scenario) : scenario;
  if (chosen === undefined) throw new Error(`no mock scenario named ${String(scenario)}`);
  // `shared` is a second window on the same main: both hear what either one's Fleet publishes.
  const api = shared ?? fakeBridge(chosen);
  window.armada = api;
  const root = createRoot(host);
  let say = (): void => undefined;
  const onScreen = new Promise<void>((resolve) => {
    say = resolve;
  });
  root.render(
    <StrictMode>
      <Boundary region="the window" usable={false} bridge={chosen.state.bridge}>
        {/* The shape comparison, and the mock is the only place it is provided. */}
        <GuideShapeProvider shape={guides}>
          {/* As `main.tsx` mounts it, so a test can read what a press asked the trackpad to play. */}
          <HapticsProvider perform={(pattern) => api.tap(pattern)}>
            {/* What this moment holds that Fleet cannot serve yet. The app's own
                mount provides none, so every field is absent there. The context
                is what a composer reads before a Job exists; the prop is what a
                Job's own boards read. */}
            <DraftedFrom held={chosen.draft ?? {}}>
              <App {...(chosen.draft === undefined ? {} : { draft: chosen.draft })} />
            </DraftedFrom>
            <OnScreen say={say} />
          </HapticsProvider>
        </GuideShapeProvider>
      </Boundary>
    </StrictMode>,
  );
  return { api, scenario: chosen, onScreen, unmount: () => root.unmount() };
}
