import { Fragment, type HTMLAttributes, type ReactNode } from "react";

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
 * with " + " between the keys — `Kbd`'s own box, holding more than one key.
 *
 * **This reverses the prior rule.** The rule used to be one key per box,
 * because `⌘K` set in a single box read as a key that did not exist,
 * separated by a gap rather than a `+`. The owner reversed it 2026-09-16
 * against a reference image reading "Ctrl + B" as a single pill: two boxes
 * with a gap between them read as two keystrokes taken one after another,
 * which is the wrong claim for a combination — the "+"-joined single box is
 * what reads as keys held down together, and does not read as a key called
 * "Ctrl + B" any more than the two-box shape avoided reading as one.
 */
export function KbdChord({
  keys,
  className,
  ...rest
}: { keys: ReactNode[] } & Omit<HTMLAttributes<HTMLElement>, "children">) {
  return (
    <Kbd className={className} {...rest}>
      {keys.map((key, i) => (
        <Fragment key={i}>
          {i > 0 ? " + " : null}
          {key}
        </Fragment>
      ))}
    </Kbd>
  );
}

/**
 * A Global-tier binding of the shape `⌘X` — every one of them but
 * `bridge_surfaces` and `history`, which already spell a range or a pair —
 * drawn as one box, `⌘` and the key joined by " + ". A caller with a range
 * or a pair to draw still owns its own cut.
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
