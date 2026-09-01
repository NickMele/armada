import type { LucideIcon } from "lucide-react";

import type { Rendering } from "../generated/vocabulary";

/**
 * How a screen's fixture states a badge: by naming the variant, never by
 * writing what it says.
 *
 * **Nothing here picks a verb, a glyph or a hue.** All three are the registry's,
 * emitted into this package at `src/generated/vocabulary.ts` from
 * `enum-verbs.toml`. That is why a fixture can state a state without stating
 * its copy — and why two stories carrying two escalation reasons cannot both be
 * satisfied by one string typed in, which is how job detail came to draw
 * `Needs you` over every escalation there is. #294.
 *
 * Beside the fixtures rather than inside one, because two screen files draw a
 * job detail header and a second copy of this is how they start to disagree.
 */

/** The glyph slot where the registry names none. `Badge` draws no stand-in. */
const NO_GLYPH: LucideIcon = undefined as unknown as LucideIcon;

/**
 * A registry verb, opening the badge. Capitalising the first letter where one
 * leads a label is presentation, not a second spelling — the same rule as
 * `leading` in Bridge's `JobDetail.tsx`, spelled twice because the two surfaces
 * share a registry and no module. Reported.
 */
function leading(verb: string): string {
  return verb.charAt(0).toUpperCase() + verb.slice(1);
}

/**
 * The three things a badge is, from the map that owns them.
 *
 * A gap renders as the gap: the wire spelling stands in for a missing verb,
 * which is recoverable, and a missing glyph draws nothing. That is `Reading`'s
 * fallback in Bridge's `reading.ts`, reached here over the same map.
 */
export function badgeOf(named: string, from: Readonly<Record<string, Rendering | undefined>>) {
  const reads = from[named];
  return {
    status: reads?.badgeStatus ?? named,
    statusIcon: reads?.icon ?? NO_GLYPH,
    statusLabel: leading(reads?.verb ?? named),
  };
}
