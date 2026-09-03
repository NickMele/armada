// The application's layout, and what catches it when it fails.
//
// **A shell holds a screen; it is not one.** The rail, the head, the status
// bar, the toast region and the error boundary are the same whatever surface is
// open, which is what makes them a layer rather than part of the first screen
// that needed them.
//
// It knows the wire and the components and nothing above: a shell that reached
// into a screen would be a layout deciding what it contains.

export * from "./Boundary";
export * from "./ClearTerminal";
export * from "./CopiedToast";
export * from "./failures";
export * from "./FailureSurface";
export * from "./fleet";
export * from "./floor";
export * from "./Head";
export * from "./Palette";
export * from "./Shell";
export * from "./uncaught";
