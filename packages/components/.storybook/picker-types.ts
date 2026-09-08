// Shared between manager.tsx (sends the start/cancel gestures) and picker.ts
// (does the pointing, in the preview iframe) so the two sides of the channel
// agree on event names and payload shape without importing each other.

export const PICK_START = "armada/picker/start";
export const PICK_CANCEL = "armada/picker/cancel";
export const PICK_RESULT = "armada/picker/result";
export const PICK_CANCELLED = "armada/picker/cancelled";

export interface PickedElement {
  classes: string[];
  selector: string;
  text: string;
  tag: string;
}

export interface PickedRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface Picked {
  mode: "element" | "region";
  rect: PickedRect;
  elements: PickedElement[];
  // Present only when the region held more than the cap. A box dragged
  // around half the screen must not write hundreds of elements into one line.
  elided?: number;
}
