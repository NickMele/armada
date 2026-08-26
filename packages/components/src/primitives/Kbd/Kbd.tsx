import type { HTMLAttributes } from "react";

/**
 * kbd — the one non-shadcn primitive. It appears in command palette rows, in
 * dropdown-menu items and in tooltips where the action has a binding.
 *
 * Never `--fg-default`. A shortcut hint is reference material sitting beside
 * the thing it describes, and rendering it at full contrast makes it compete
 * with the label it belongs to.
 *
 * The palette is the discovery surface — it is how a person learns forty
 * shortcuts without a cheat sheet — so every entry displays its binding, and
 * this is what displays it.
 */
export type KbdProps = HTMLAttributes<HTMLElement>;

export function Kbd({ className, ...rest }: KbdProps) {
  return <kbd className={className ? `armada-kbd ${className}` : "armada-kbd"} {...rest} />;
}

/**
 * A chord renders as one key per element rather than one string, because
 * `⌘K` set in a single box reads as a key that does not exist. Separated by
 * a gap, not by a `+`.
 */
export function KbdChord({ className, ...rest }: HTMLAttributes<HTMLSpanElement>) {
  return (
    <span className={className ? `armada-kbd-chord ${className}` : "armada-kbd-chord"} {...rest} />
  );
}
