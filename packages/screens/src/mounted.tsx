// Mounting a screen in the browser project, and taking it down again.
//
// **`createRoot` and nothing else.** A render helper from a testing library
// would be a fourth thing between the screen and the assertion, and the one
// thing this package's browser tests exist to avoid is asserting against
// something that imitates the app — `packages/components/src/screens/` is a
// frame built from primitives, and the reason these tests are here rather than
// there is that it is not this package's screens.
//
// The stylesheet is deliberately not loaded. What these tests read is the
// accessibility tree — a control that is disabled, a callback that did not
// fire — and none of that is a property of the CSS. A screen's appearance is
// the components package's to prove, story by story.

import type { ReactElement } from "react";
import { createRoot, type Root } from "react-dom/client";

let held: { root: Root; host: HTMLElement } | null = null;

/**
 * Put a screen on the page. **One at a time**, so a query for a role finds the
 * screen under test rather than the one before it — a mounted React root
 * survives the test that made it unless something takes it down.
 */
export function mount(screen: ReactElement): void {
  unmount();
  const host = document.createElement("div");
  document.body.append(host);
  const root = createRoot(host);
  held = { root, host };
  // Synchronous on purpose: the test's next line queries the DOM, and
  // `createRoot().render` is not.
  //
  // React 19 has no synchronous render, so this leans on `flushSync` through
  // the caller instead — the `act`-free way is to await a microtask, which is
  // what every assertion below already does through `expect.element`.
  root.render(screen);
}

/** Take it down. Called before each mount and after each test. */
export function unmount(): void {
  if (held === null) return;
  held.root.unmount();
  held.host.remove();
  held = null;
}
