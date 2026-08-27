// The error boundary `docs/practices/react.md` has required since before Bridge
// had one: a Job that cannot be rendered must not blank the window.
//
// **A boundary that catches and renders nothing is a blank screen with extra
// steps**, so the fallback is the failure notice — what broke, the component,
// the stack folded away, the log, and something to do. No run id: a render
// that threw never reached Fleet, and Bridge mints none.
//
// # What a boundary does not catch
//
// Not an event handler, not a rejected promise, not anything thrown after the
// render that mounted it, and not its own fallback. `uncaught.ts` is the other
// half; between them the renderer has no silent path left.

import { Component } from "react";
import type { ErrorInfo, ReactNode } from "react";

import type { BridgeIdentity } from "../../shared/bridge";
import type { Caught } from "./failures";
import { FailureSurface } from "./FailureSurface";

export type BoundaryProps = {
  /**
   * What this boundary was drawing, in the app's voice — "the job list", not a
   * component name. The component the stack names goes in the fold.
   */
  region: string;
  /** Whether the rest of the window survives this. The root boundary is the one that does not. */
  usable?: boolean;
  bridge: BridgeIdentity;
  /** The app's toast layer, where one survives this failure. */
  onCopied?: (value: string) => void;
  children: ReactNode;
};

type BoundaryState = { caught: Caught | null };

export class Boundary extends Component<BoundaryProps, BoundaryState> {
  override state: BoundaryState = { caught: null };

  /** Runs before `componentDidCatch`, and is what stops the blank frame. */
  static getDerivedStateFromError(thrown: unknown): BoundaryState {
    return { caught: read(thrown, null) };
  }

  /** The component stack arrives here and nowhere else. */
  override componentDidCatch(thrown: unknown, info: ErrorInfo): void {
    this.setState({ caught: read(thrown, info.componentStack ?? null) });
  }

  override render(): ReactNode {
    const caught = this.state.caught;
    if (caught === null) return this.props.children;
    return (
      <FailureSurface
        caught={caught}
        region={this.props.region}
        usable={this.props.usable ?? true}
        bridge={this.props.bridge}
        onCopied={this.props.onCopied}
      />
    );
  }
}

/** Anything can be thrown, so nothing is assumed to be an `Error`. */
function read(thrown: unknown, where: string | null): Caught {
  const message = thrown instanceof Error ? thrown.message : String(thrown);
  const stack = thrown instanceof Error ? (thrown.stack ?? null) : null;
  return { message, component: firstFrame(where), where, stack };
}

/**
 * The component React blamed. The stack's first frame, which is the one that
 * threw — every frame above it is a parent that merely contained it.
 */
function firstFrame(where: string | null): string | null {
  if (where === null) return null;
  const frame = where
    .split("\n")
    .map((line) => line.trim())
    .find((line) => line.startsWith("at "));
  if (frame === undefined) return null;
  return frame.slice("at ".length).split(" ")[0] ?? null;
}
