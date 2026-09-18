import type { KeyboardEvent } from "react";

import { keyFor } from "./actions";
import { KbdCmd } from "./primitives/Kbd/Kbd";
import { useShortcutReveal } from "./shortcut-reveal";

/**
 * `⌘Enter` sends the message box that has focus — the drone message box and
 * Helm's composer, settled together by the owner on 18 Sep 2026.
 *
 * **One binding, honoured in two places and spelled in neither.**
 * `send_message` in `crates/core-model/domain/actions.toml` is the key, and
 * these are the two readings every composer shares: the predicate and the
 * badge. Two boxes each testing `metaKey` themselves is two answers the day
 * the registry moves it.
 */
export const SEND_MESSAGE = "send_message";

/**
 * Whether this keystroke is the send.
 *
 * **Plain `Enter` is never it**: a redirect and an ask are both prose, so the
 * modifier is what separates a paragraph break from a send. Ctrl is taken too,
 * as `StudioAddNode` and `⌘K` already take it; nothing renders it.
 */
export function sendsOn(event: KeyboardEvent<HTMLElement>): boolean {
  return event.key === "Enter" && (event.metaKey || event.ctrlKey) && !event.altKey;
}

/**
 * The binding drawn inside a Send button, for as long as `⌘` is held.
 *
 * **`aria-hidden`, so Send's accessible name stays "Send"** rather than
 * growing a keycap the moment a modifier goes down.
 *
 * **Mounted on the hold**, as `TitleBar`'s Helm button is and not as the
 * sidebar's rows are: Send is positioned inside a field one dock wide, and
 * reserving the badge's width at rest would eat the well it sits over.
 *
 * **Only while the control can act** — a keycap on a disabled Send names a key
 * that does nothing.
 */
export function SendKbd({ available }: { available: boolean }) {
  const revealing = useShortcutReveal();
  if (!revealing || !available) return null;
  return (
    <span aria-hidden>
      <KbdCmd shortcut={keyFor(SEND_MESSAGE)} />
    </span>
  );
}
