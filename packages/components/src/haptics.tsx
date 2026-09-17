import { createContext, useContext, useEffect, useRef, type ReactNode } from "react";
import type { ButtonAnswer } from "./primitives/Button/Button";

/**
 * The two trackpad patterns a press can be answered with. The rule is
 * `docs/contracts/design-system.md`, under Touch: Alignment for accepted, Level
 * change for refused, and nothing but a person's own press plays either.
 */
export type HapticPattern = "alignment" | "level_change";

/** Play one pattern, and never wait for it. */
export type PerformHaptic = (pattern: HapticPattern) => void;

/**
 * A no-op where nothing provides one: Storybook, the browser mock, and any
 * window with no trackpad to reach. Components stay pure, and only Bridge's
 * renderer entry supplies a performer.
 */
const Haptics = createContext<PerformHaptic>(() => undefined);

export function HapticsProvider({ perform, children }: { perform: PerformHaptic; children: ReactNode }) {
  return <Haptics.Provider value={perform}>{children}</Haptics.Provider>;
}

export function useHaptics(): PerformHaptic {
  return useContext(Haptics);
}

/**
 * Plays the answer's pattern once, on the render a control that was waiting on
 * Fleet shows it.
 *
 * **Only a control that was itself pending.** An answer a control mounts with,
 * or one set on it without a press, replays the line and plays nothing: a
 * second control drawing the same answer would otherwise tap the trackpad
 * twice for one press.
 */
export function useAnswerTap(pending: boolean, answer: ButtonAnswer | undefined): void {
  const perform = useHaptics();
  const waited = useRef(false);
  useEffect(() => {
    if (pending) {
      waited.current = true;
      return;
    }
    if (waited.current && answer !== undefined) {
      perform(answer === "accepted" ? "alignment" : "level_change");
    }
    waited.current = false;
  }, [pending, answer, perform]);
}
