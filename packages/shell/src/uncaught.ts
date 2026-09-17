// The failures an error boundary does not catch.
//
// **A boundary catches a render and nothing else.** A throw inside a click
// handler, and a rejected promise from a `void`-ed preload call, both reach
// nothing and leave the window looking as though the click did not land — which
// is the same silence, arriving by a different route.
//
// Bridge fires two `void`-ed calls today, `proposeJob` and `approveDispatch`.
// A rejected `ipcRenderer.invoke` on either is a button that does nothing and
// says nothing.

export type Uncaught = {
  /** Which listener saw it. A rejection and a throw are not the same thing. */
  from: "throw" | "rejection";
  message: string;
  stack: string | null;
};

/**
 * The notices every engine sends down `window.onerror` that are not failures.
 *
 * **A `ResizeObserver` delivers its callbacks in a loop inside the frame.**
 * Where a callback changes layout the loop runs out of rounds with observations
 * still pending, and the browser says so and delivers them on the next frame.
 * Nothing threw, nothing was lost and nothing stopped — which is exactly what
 * the rest of this module is for, and this is none of it.
 *
 * The wording is the engine's, so both are named. Chromium's is what Bridge
 * runs on; WebKit's and Firefox's reach the dev mock, which is a browser.
 */
const NOTICES: readonly string[] = [
  // Chromium, and so Electron.
  "ResizeObserver loop completed with undelivered notifications.",
  // WebKit and Firefox, for the same event.
  "ResizeObserver loop limit exceeded",
];

/**
 * **Both halves are required.** The message alone is a string anyone may throw,
 * and a real `Error` carrying this sentence is a throw like any other and is
 * still reported. What marks a notice is that there is no exception behind it:
 * `event.error` is null and `event.message` is the whole of what happened.
 *
 * This is the only thing dropped here, and it is dropped by name. **Nothing
 * else is** — a window that fails in silence is worse than one that says so.
 */
function isNotice(event: ErrorEvent): boolean {
  return !(event.error instanceof Error) && NOTICES.includes(event.message);
}

/**
 * Watch for both, and return the unsubscribe.
 *
 * The listeners do not `preventDefault`: whatever the platform does with an
 * uncaught failure it keeps doing, and this only adds a surface that says so.
 *
 * Everything reaching either listener is reported but {@link NOTICES}, which
 * are the browser talking about itself rather than anything Bridge did.
 */
export function watchUncaught(onCaught: (uncaught: Uncaught) => void): () => void {
  const threw = (event: ErrorEvent): void => {
    if (isNotice(event)) return;
    onCaught({
      from: "throw",
      message: event.message === "" ? String(event.error) : event.message,
      stack: event.error instanceof Error ? (event.error.stack ?? null) : null,
    });
  };

  const rejected = (event: PromiseRejectionEvent): void => {
    const reason: unknown = event.reason;
    onCaught({
      from: "rejection",
      message: reason instanceof Error ? reason.message : String(reason),
      stack: reason instanceof Error ? (reason.stack ?? null) : null,
    });
  };

  window.addEventListener("error", threw);
  window.addEventListener("unhandledrejection", rejected);
  return () => {
    window.removeEventListener("error", threw);
    window.removeEventListener("unhandledrejection", rejected);
  };
}
