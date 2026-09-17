import { createContext, useContext, useEffect, useState, type ReactNode } from "react";

/**
 * Whether Cmd is down right now, so a control carrying a Global-tier binding
 * can show its own `Kbd` badge without a person opening the palette to learn
 * it exists. **Global tier only** — `⌘K`, `⌘1–⌘8`, `⌘J`, `⌘\`, `⌘[ ⌘]`, per
 * `docs/contracts/design-system.md`'s "Two tiers". `Esc` takes no modifier
 * and every Contextual act (`j`, `x`, `n`, …) is single-key; holding Cmd does
 * not fire either kind, so revealing them here would tell a person to press a
 * key that means something else without Cmd down.
 *
 * **One listener, one boolean, read through context** rather than a prop
 * threaded from `App.tsx` down through `Shell` and `TheShell` to every
 * control that carries a binding — the same reasoning `RovingOption`
 * (`JobRowStacked.tsx`) reads a list's own position through context instead
 * of a prop passed to every row.
 */
const ShortcutReveal = createContext(false);

/** `true` for exactly as long as Cmd is held, and never true where nothing provides it. */
export function useShortcutReveal(): boolean {
  return useContext(ShortcutReveal);
}

/**
 * Mounted once, in `TheShell` — the one ancestor every control this reveals
 * shares. `TitleBar`'s search field and `Chord` in `TheShell.tsx` already show
 * their own binding at all times and are unchanged by this; this is only for
 * the controls that had no badge to show before it.
 *
 * **`keydown`/`keyup` on `Meta`, the same shape `App.tsx`'s own Escape
 * listener uses**, plus the three ways a hold ends without a `keyup` ever
 * reaching this window: releasing Cmd over a native menu, Cmd-Tab switching
 * applications, and the window itself losing focus. A `keydown` never
 * inserts a character on its own, so this fires the same way whether or not
 * a text field holds focus — nothing here suppresses typing, and nothing
 * here suppresses the shortcut a person is actually pressing.
 */
export function ShortcutRevealProvider({ children }: { children: ReactNode }) {
  const [revealing, setRevealing] = useState(false);

  useEffect(() => {
    function pressed(event: KeyboardEvent): void {
      if (event.key === "Meta") setRevealing(true);
    }
    function released(event: KeyboardEvent): void {
      if (event.key === "Meta") setRevealing(false);
    }
    function clear(): void {
      setRevealing(false);
    }
    function visibility(): void {
      if (document.hidden) clear();
    }
    window.addEventListener("keydown", pressed);
    window.addEventListener("keyup", released);
    window.addEventListener("blur", clear);
    document.addEventListener("visibilitychange", visibility);
    return () => {
      window.removeEventListener("keydown", pressed);
      window.removeEventListener("keyup", released);
      window.removeEventListener("blur", clear);
      document.removeEventListener("visibilitychange", visibility);
    };
  }, []);

  return <ShortcutReveal.Provider value={revealing}>{children}</ShortcutReveal.Provider>;
}
