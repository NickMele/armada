// The two labels every region on this screen draws: its band, and a field's own
// name. Both look their sentence up in `concepts.ts` rather than writing one,
// because a screen that answered would be a second place the vocabulary lives.

import type { ReactNode } from "react";
import { Tooltip, conceptSaid } from "@armada/components";

/**
 * A region's band, and what that region is where its name is an Armada word.
 *
 * **The sentence is looked up rather than written here.** Six regions on this
 * screen name a thing rather than describe it — `Brief`, `The run`, `Where
 * things are` — and a screen that answered them itself would be a second place
 * the vocabulary is explained. `concepts.ts` is the first.
 *
 * `asChild`, because the brief's own rule keys off the band's class: a wrapper
 * would be the `:not(.armada-screen__eyebrow)` child and be styled as the
 * brief itself.
 */
export function Eyebrow({ children, spaced }: { children: ReactNode; spaced?: boolean }) {
  const says = conceptSaid(children);
  const band = (
    <span className="armada-screen__eyebrow" data-spaced={spaced || undefined}>
      {children}
    </span>
  );
  return says === undefined ? (
    band
  ) : (
    <Tooltip asChild label={says}>
      {band}
    </Tooltip>
  );
}

/** A step field's label, on the same rule as the header's and the tree's. */
export function FieldLabel({ children }: { children: ReactNode }) {
  const says = conceptSaid(children);
  const label = <span className="armada-inside__field-label">{children}</span>;
  return says === undefined ? (
    label
  ) : (
    <Tooltip asChild label={says}>
      {label}
    </Tooltip>
  );
}
