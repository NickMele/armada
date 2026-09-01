import { useEffect, useRef, useState } from "react";

import "./Tokens.css";

/**
 * A specimen of one group of design tokens, read off the running stylesheet.
 *
 * **It reads the value rather than restating it.** Every swatch resolves its
 * own custom property at paint, so a specimen cannot say one thing while
 * `packages/tokens` says another. The alternative — a table of hexes beside the
 * file that declares them — is a second copy of the palette, and a second copy
 * is the thing this exists to replace.
 *
 * Grouped the way `docs/contracts/design-system.md` groups them, because the
 * contract is what a person reads next and a specimen that ordered them
 * differently would be a third arrangement to hold in mind.
 */
export type TokensProps = {
  /** What the group is called, as the contract calls it. */
  label: string;
  /** Why the group exists, in one line. */
  note?: string;
  /** The custom properties in it, without the leading dashes. */
  names: readonly string[];
  /**
   * How each one is shown. `fill` paints the value, `text` sets type in it,
   * `size` draws a bar that long, and `duration` moves a mark for that long.
   */
  as?: "fill" | "text" | "size" | "duration";
};

export function Tokens({ label, note, names, as = "fill" }: TokensProps) {
  const host = useRef<HTMLDivElement>(null);
  const [resolved, setResolved] = useState<Record<string, string>>({});

  // After paint, because a custom property has no value until something is in
  // the document to inherit it.
  useEffect(() => {
    const el = host.current;
    if (el === null) return;
    const style = getComputedStyle(el);
    setResolved(Object.fromEntries(names.map((n) => [n, style.getPropertyValue(`--${n}`).trim()])));
  }, [names]);

  return (
    <section className="armada-tokens" ref={host}>
      <header className="armada-tokens__head">
        <h3 className="armada-tokens__label">{label}</h3>
        {note === undefined ? null : <p className="armada-tokens__note">{note}</p>}
      </header>
      <dl className="armada-tokens__list">
        {names.map((name) => (
          <div className="armada-tokens__row" key={name}>
            <dt className="armada-tokens__name">--{name}</dt>
            <dd className="armada-tokens__value">
              <Sample as={as} name={name} />
              {/* The resolved value, and never a hand-typed one. Empty until
                  the effect has read it, which is one frame and says nothing
                  false in the meantime. */}
              <span className="armada-tokens__resolved">{resolved[name] ?? ""}</span>
            </dd>
          </div>
        ))}
      </dl>
    </section>
  );
}

/** One token, shown the way its kind is legible. */
function Sample({ as, name }: { as: TokensProps["as"]; name: string }) {
  const value = `var(--${name})`;
  if (as === "text") {
    return (
      <span className="armada-tokens__type" style={{ font: value, fontSize: value }}>
        The quick brown fox
      </span>
    );
  }
  if (as === "size") {
    return <span className="armada-tokens__bar" style={{ width: value }} />;
  }
  if (as === "duration") {
    return <span className="armada-tokens__mark" style={{ transitionDuration: value }} />;
  }
  return <span className="armada-tokens__chip" style={{ background: value }} />;
}
