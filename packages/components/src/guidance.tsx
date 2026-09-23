import { createContext, useCallback, useContext, useRef, useState, type ReactNode } from "react";

import { GuideCard } from "./compositions/GuideCard/GuideCard";
import type { Guide } from "./guides/guide";

/**
 * What a window remembers about the guides.
 *
 * **The piece is what is remembered, never the number.** A guide renumbered in
 * the catalogue is the same piece and stays met; a piece renamed is a new
 * piece, and everybody gets their first contact with it back.
 */
export type GuidanceMemory = {
  /** First contact is off. Nothing opens by itself while this is true. */
  off: boolean;
  /** Pieces already met. A piece in here never opens itself again. */
  met: readonly string[];
  /** Whether any card has ever been on screen. The first one carries the switch. */
  cardSeen: boolean;
};

const NOTHING_MET: GuidanceMemory = { off: false, met: [], cardSeen: false };

// `localStorage` rather than a Fleet preference, `left-collapsed.ts`' own
// reason: this is one window's way of looking, not a fact about the fleet.
const KEY = "armada.bridge.guides";

function readMemory(): GuidanceMemory {
  try {
    const held = window.localStorage.getItem(KEY);
    if (held === null) return NOTHING_MET;
    const parsed = JSON.parse(held) as Partial<GuidanceMemory>;
    return {
      off: parsed.off === true,
      met: Array.isArray(parsed.met) ? parsed.met.filter((one) => typeof one === "string") : [],
      cardSeen: parsed.cardSeen === true,
    };
  } catch {
    // Unreadable is the same answer as nothing stored: every guide is unmet.
    return NOTHING_MET;
  }
}

function writeMemory(memory: GuidanceMemory): void {
  try {
    window.localStorage.setItem(KEY, JSON.stringify(memory));
  } catch {
    // A failed write leaves the choice unremembered, the honest answer for a preference.
  }
}

/** What a mark, the catalogue and Settings can each ask of the guidance system. */
export type Guidance = {
  /** Open a guide because somebody pressed for it. */
  onOpen: (guide: Guide) => void;
  /**
   * Say a piece is on a screen the person is looking at. Opens its card once,
   * the first time, unless first contact is off.
   */
  onMet: (guide: Guide) => void;
  /** First contact is off. The switch on the first card, and the one in Settings. */
  off: boolean;
  onOff: (off: boolean) => void;
};

/**
 * A no-op where nothing provides one: a story that draws a mark without
 * wanting the layer, and anything mounted outside the window. `haptics.tsx`
 * sets the precedent — components stay pure, and one provider supplies the
 * behaviour.
 */
const GuidanceContext = createContext<Guidance>({
  onOpen: () => undefined,
  onMet: () => undefined,
  off: false,
  onOff: () => undefined,
});

export function useGuidance(): Guidance {
  return useContext(GuidanceContext);
}

/** What is on screen, decided when it opens so that reading it changes nothing. */
type Showing = {
  guide: Guide;
  /** Somebody pressed for it. An uninvited card does not animate in. */
  invited: boolean;
  /** The first card anybody has ever seen, so it carries the switch. */
  withSwitch: boolean;
};

export type GuidanceProviderProps = {
  children: ReactNode;
  /**
   * Whether the choice survives the window closing. `false` is for a story: a
   * story that wrote to this window's storage would decide what the next one
   * has already met.
   */
  remembered?: boolean;
};

/**
 * Holds what has been met, opens the card, and is the one place first contact
 * is decided.
 *
 * **At most one card opens by itself per session, and this is the cost that
 * buys it.** A screen carrying four pieces nobody has met opens one card, not
 * four; the rest keep their first contact for a later visit. Four uninvited
 * cards in a row is the noise the mark exists to replace.
 */
export function GuidanceProvider({ children, remembered = true }: GuidanceProviderProps) {
  const [memory, setMemory] = useState<GuidanceMemory>(() => (remembered ? readMemory() : NOTHING_MET));
  const [showing, setShowing] = useState<Showing | null>(null);

  // Read through a ref, so the two callbacks below never change identity: a
  // mark says it is on screen from an effect, and an effect that re-runs on
  // every render of the provider would say it on every event Fleet publishes.
  const latest = useRef({ memory, showing, remembered });
  latest.current = { memory, showing, remembered };
  // One self-opened card per session. Not stored: a person who reopens the
  // window is meeting the next piece for the first time all over again.
  const openedItself = useRef(false);

  const remember = useCallback((next: GuidanceMemory) => {
    setMemory(next);
    if (latest.current.remembered) writeMemory(next);
  }, []);

  const onOpen = useCallback(
    (guide: Guide) => {
      const held = latest.current.memory;
      setShowing({ guide, invited: true, withSwitch: !held.cardSeen });
      remember({ ...held, cardSeen: true, met: metWith(held.met, guide.piece) });
    },
    [remember],
  );

  const onMet = useCallback(
    (guide: Guide) => {
      const { memory: held, showing: open } = latest.current;
      if (held.off || open !== null || openedItself.current) return;
      if (held.met.includes(guide.piece)) return;
      openedItself.current = true;
      setShowing({ guide, invited: false, withSwitch: !held.cardSeen });
      remember({ ...held, cardSeen: true, met: metWith(held.met, guide.piece) });
    },
    [remember],
  );

  const onOff = useCallback(
    (off: boolean) => remember({ ...latest.current.memory, off }),
    [remember],
  );

  return (
    <GuidanceContext.Provider value={{ onOpen, onMet, off: memory.off, onOff }}>
      {children}
      {showing === null ? null : (
        <GuideCard
          guide={showing.guide}
          invited={showing.invited}
          onClose={() => setShowing(null)}
          {...(showing.withSwitch ? { off: memory.off, onOff } : {})}
        />
      )}
    </GuidanceContext.Provider>
  );
}

function metWith(met: readonly string[], piece: string): readonly string[] {
  return met.includes(piece) ? met : [...met, piece];
}
