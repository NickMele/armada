// The capture window's own shapes — #1294, `docs/practices/capture-window.md`.
//
// **Two documents and one of them is not Armada's.** The window shows another
// repository's running web app in a view with no preload, and Bridge's bar
// above it in a view that has Bridge's preload and Bridge's CSP. Everything
// here crosses between main and that bar, which is Bridge on both ends; what
// crosses between main and the page is `StudioCapture` alone, and main
// re-applies every bound on it.
//
// **Nothing here names the Studio's id to the page.** The bar holds it because
// the bar is Armada's own document; the page is told nothing about a Studio, a
// port or a path.

import type { CaptureServed } from "@armada/protocol";

/** A rectangle in the page's viewport, in CSS pixels. */
export type CaptureRect = { x: number; y: number; width: number; height: number };

/** Which hold refused an address, so the bar can say what was refused rather than that something was. */
export type CaptureRefusedWhat = "navigation" | "redirect" | "frame" | "window" | "download";

/** An address this window would not follow, drawn in full and never followed. */
export type CaptureRefusal = {
  address: string;
  what: CaptureRefusedWhat;
  /** Whether the system browser would take it — `http:` and `https:` only. */
  offerable: boolean;
};

/**
 * What the bar draws. **Pushed by main after every change**, so the bar holds
 * no state of its own about the Run, the address or the Studio.
 */
export type CaptureWindowState = {
  /** The Run, as the Note will record it. */
  served: CaptureServed;
  /** The Studio a Note lands on, decided when the window opened. `null` names an untitled one. */
  studio: { id: string; name: string | null };
  /**
   * Whether the Run is still serving. **Capture is refused from the moment it
   * is not**, and the window loads nothing further.
   */
  serving: boolean;
  /** Whether capture is armed: the page is held still and a press points. */
  armed: boolean;
  /** How many subframe navigations were refused since the window opened. */
  framesRefused: number;
  /** The last address refused, or absent. */
  refused?: CaptureRefusal;
};

/** What the bar draws over the element under the pointer while capture is armed. */
export type CaptureAimed = { box: CaptureRect; named: string };

/**
 * The element a note is being written on. **The capture itself stays in main**:
 * the bar gets the box to draw and the one line that names what was hit, and
 * the markup, the styles and the selector never leave the process that bounded
 * them.
 */
export type CaptureHeld = { box: CaptureRect; chain: string };

/** A wheel the bar took while armed, forwarded so the page still scrolls under it. */
export type CaptureWheel = { x: number; y: number; deltaX: number; deltaY: number };
