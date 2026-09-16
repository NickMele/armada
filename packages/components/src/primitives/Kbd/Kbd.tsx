import type { HTMLAttributes, ReactNode } from "react";

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
 * A chord is every key of one binding pressed together, drawn as **one box**
 * — `Kbd`'s own box, holding more than one key. That "one box" call is from
 * 2026-09-16, against two boxes reading as keys pressed in sequence.
 *
 * **No " + " between them.** Two text nodes in one line box do not share a
 * baseline — `⌘` sat higher than the letter beside it — and a literal `+`
 * read as a third key. Each key is its own flex child instead, spaced by
 * `gap` on the box, so `align-items` centers every key on its own.
 */
export function KbdChord({
  keys,
  className,
  ...rest
}: { keys: ReactNode[] } & Omit<HTMLAttributes<HTMLElement>, "children">) {
  return (
    <Kbd className={className} {...rest}>
      {keys.map((key, i) => (
        <span className="armada-kbd__key" key={i}>
          {key}
        </span>
      ))}
    </Kbd>
  );
}

/**
 * A Global-tier binding of the shape `⌘X` — every one of them but
 * `bridge_surfaces` and `history`, which already spell a range or a pair —
 * drawn as one box, `⌘` and the key as separate flex children. A caller
 * with a range or a pair to draw still owns its own cut.
 */
export function KbdCmd({ shortcut }: { shortcut: string }) {
  return <KbdChord keys={["⌘", shortcut.slice(1)]} />;
}

/**
 * `binding` drawn as a chord when it is the `⌘X` shape `KbdCmd` recognizes,
 * and as a plain single key otherwise — for a caller such as `Sheet` that
 * draws whatever binding it is given without knowing in advance whether it
 * is a combination or a single key such as `Esc`.
 */
export function KbdBinding({ binding }: { binding: string }) {
  return binding.length > 1 && binding.startsWith("⌘") ? (
    <KbdCmd shortcut={binding} />
  ) : (
    <Kbd>{binding}</Kbd>
  );
}
