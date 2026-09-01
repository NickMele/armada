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
 * Watch for both, and return the unsubscribe.
 *
 * The listeners do not `preventDefault`: whatever the platform does with an
 * uncaught failure it keeps doing, and this only adds a surface that says so.
 */
export function watchUncaught(onCaught: (uncaught: Uncaught) => void): () => void {
  const threw = (event: ErrorEvent): void => {
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
