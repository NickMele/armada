// A person's UI choices that outlive one Job and one launch of Bridge — not
// Fleet's concern, and too small to invent a settings channel for. Nothing
// reaches the renderer from `crates/config/settings.toml` yet (`board.ts`'s
// own comment, on the Board's sort default), so this reaches straight for
// the renderer's own persistence: `localStorage`, under one prefix so a key
// collision elsewhere in the app is visible rather than silent.

import { useCallback, useState } from "react";

const PREFIX = "armada.";

/** A remembered boolean, or `null` where nothing was ever written. */
export function rememberedBoolean(key: string): boolean | null {
  try {
    const raw = localStorage.getItem(PREFIX + key);
    return raw === null ? null : raw === "true";
  } catch {
    // A blocked store — private browsing, a locked-down profile — reads as
    // nothing remembered rather than throwing through a render.
    return null;
  }
}

function remember(key: string, value: boolean): void {
  try {
    localStorage.setItem(PREFIX + key, String(value));
  } catch {
    // Not worth surfacing over one flag.
  }
}

/**
 * A boolean choice, read once on mount and written back on every change.
 * `fallback` is what somebody who has never chosen gets.
 */
export function useRememberedBoolean(
  key: string,
  fallback: boolean,
): [boolean, (value: boolean) => void] {
  const [value, setValue] = useState(() => rememberedBoolean(key) ?? fallback);
  const set = useCallback(
    (next: boolean) => {
      setValue(next);
      remember(key, next);
    },
    [key],
  );
  return [value, set];
}
