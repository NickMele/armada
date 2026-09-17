import { createContext, useContext, type ReactNode } from "react";
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
 * Which pattern answers a press, and the whole of the mapping.
 *
 * **Played where Fleet's answer arrives, never by the control that drew it.**
 * A control is often gone by then — an accepted Forget leaves no row, and an
 * event can swap a control while its act is still out — and a tap tied to that
 * control's own render is lost in exactly those cases. #1326.
 */
export function patternFor(answer: ButtonAnswer): HapticPattern {
  return answer === "accepted" ? "alignment" : "level_change";
}
