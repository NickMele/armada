import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import type { FocusEvent, KeyboardEvent, PointerEvent, RefObject } from "react";

/**
 * What makes a control confirm in place: a press that commits only once held
 * for `--duration-hold`. The rule is the design system contract's, under Safety
 * rules for single-key actions, "A hold confirms in place".
 *
 * **One hook, two faces.** `HoldButton` is a kill drawn alone and
 * `SplitButton`'s face is a kill drawn with a menu beside it. Both read the
 * hold from here, so releasing early, leaving, auto-repeat and the reduced
 * motion fallback cannot come to mean two things.
 */

const REDUCED = "(prefers-reduced-motion: reduce)";

function subscribeToMotion(onChange: () => void): () => void {
  const query = window.matchMedia(REDUCED);
  query.addEventListener("change", onChange);
  return () => query.removeEventListener("change", onChange);
}

function motionReduced(): boolean {
  return window.matchMedia(REDUCED).matches;
}

/** Whether a person asked for less motion, following the preference as it changes. */
export function usePrefersReducedMotion(): boolean {
  return useSyncExternalStore(subscribeToMotion, motionReduced, () => false);
}

/**
 * `--duration-hold` on `element`, in milliseconds. `null` where it is absent,
 * unparseable or zero: a hold with no length is not one, so the press asks.
 */
export function holdDurationOf(element: Element): number | null {
  const raw = getComputedStyle(element).getPropertyValue("--duration-hold").trim();
  const read = /^(\d+(?:\.\d+)?)(ms|s)$/.exec(raw);
  if (read === null) return null;
  const ms = Number(read[1]) * (read[2] === "s" ? 1000 : 1);
  return ms > 0 ? ms : null;
}

/** `rest` has no transition, `arming` fills across the hold, `releasing` empties fast. */
export type HoldPhase = "rest" | "arming" | "releasing";

function isHoldKey(key: string): boolean {
  return key === " " || key === "Enter";
}

export type HoldHandlers<T extends HTMLElement> = {
  onPointerDown: (event: PointerEvent<T>) => void;
  onPointerUp: () => void;
  onPointerLeave: () => void;
  onPointerCancel: () => void;
  onKeyDown: (event: KeyboardEvent<T>) => void;
  onKeyUp: (event: KeyboardEvent<T>) => void;
  onBlur: (event: FocusEvent<T>) => void;
};

export type Hold<T extends HTMLElement> = {
  /**
   * Whether the hold is offered at all. **Not under reduced motion**: the fill
   * is the only thing that says how long is left, so the control asks instead
   * and a caller spreads none of `handlers`.
   */
  offered: boolean;
  phase: HoldPhase;
  /** The element `--duration-hold` is read from, when a hold starts. */
  ref: RefObject<T | null>;
  /** No `onClick` among them: a click is a press and release, and a press alone never commits. */
  handlers: HoldHandlers<T>;
};

export function useHold<T extends HTMLElement>({
  inert,
  onCommit,
  onAsk,
}: {
  /** Disabled or pending. A hold does not start on, or survive, a control that is off. */
  inert: boolean;
  /** Held for the whole duration. Called once per hold. */
  onCommit: () => void;
  /** Pressed where `--duration-hold` cannot be read. Opens the confirmation. */
  onAsk: () => void;
}): Hold<T> {
  const reduced = usePrefersReducedMotion();
  const element = useRef<T>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [phase, setPhase] = useState<HoldPhase>("rest");

  function start(): void {
    if (inert || timer.current !== null || element.current === null) return;
    const duration = holdDurationOf(element.current);
    if (duration === null) {
      onAsk();
      return;
    }
    setPhase("arming");
    timer.current = setTimeout(() => {
      timer.current = null;
      setPhase("rest");
      onCommit();
    }, duration);
  }

  function cancel(): void {
    if (timer.current === null) return;
    clearTimeout(timer.current);
    timer.current = null;
    setPhase("releasing");
  }

  // A hold does not survive the control going off or the hold stopping being offered.
  useEffect(() => {
    if (inert || reduced) cancel();
  }, [inert, reduced]);
  useEffect(
    () => () => {
      if (timer.current !== null) clearTimeout(timer.current);
    },
    [],
  );

  return {
    offered: !reduced,
    phase,
    ref: element,
    handlers: {
      onPointerDown: (event) => {
        if (event.button !== 0) return;
        start();
      },
      onPointerUp: cancel,
      onPointerLeave: cancel,
      onPointerCancel: cancel,
      onKeyDown: (event) => {
        if (!isHoldKey(event.key)) return;
        // Space would scroll and Enter would click. Auto-repeat must not restart the hold.
        event.preventDefault();
        if (event.repeat) return;
        start();
      },
      onKeyUp: (event) => {
        if (!isHoldKey(event.key)) return;
        event.preventDefault();
        cancel();
      },
      onBlur: cancel,
    },
  };
}
