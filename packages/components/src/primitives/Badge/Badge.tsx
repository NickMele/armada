import type { LucideIcon } from "lucide-react";
import type { ReactNode } from "react";

/**
 * A chip is a status. A bordered pill is a Job state and nothing else — origin
 * tags, drift states and provenance are plain sans text in `--fg-muted`, so
 * they never reach this component.
 *
 * The badge has no leading dot. With an icon mandatory on every state, the dot
 * was a second marker for one claim.
 */
export type BadgeProps = {
  /**
   * The status token stem, e.g. `running`, `awaiting-review`. Drives hue and
   * tint together: `--status-{stem}` and `--status-{stem}-bg`.
   *
   * Typed as a string rather than a union because the roster is the Job state
   * machine's, and the long-term intent recorded on the contract is to
   * generate both the token names and this type from `core-model`'s enum. A
   * union hand-written here would be a third copy of that roster.
   */
  status: string;
  /**
   * The glyph, required on every state. Redundant encoding, not decoration:
   * hue alone fails under deuteranopia, on a miscalibrated monitor and in a
   * screenshot, and several statuses share one hue and are told apart by
   * glyph. Take it from `packages/icons/icons.toml`, group `Job state`.
   */
  icon: LucideIcon;
  /**
   * The verb, from the enum→verb map. Never written by hand at a call site
   * that ships — the stories write them because the map is not generated yet.
   */
  children: ReactNode;
  /**
   * The running mark, still working. One per screen, on the focused row of a
   * list, and never where a workflow rail carries a more specific mark. Only
   * the inner dot of `circle-dot` moves, so no row shifts.
   */
  pulsing?: boolean;
};

/** Badge icons are 12px at strokeWidth 2 — an exact half of lucide's 24 grid. */
const BADGE_ICON = 12;
const BADGE_STROKE = 2;

export function Badge({ status, icon: Icon, children, pulsing = false }: BadgeProps) {
  // Only the running mark pulses. Motion carries "still working", which is a
  // claim no other state makes.
  const animates = pulsing && status === "running";
  return (
    <span
      className="armada-badge"
      data-status={status}
      data-pulsing={animates || undefined}
      style={{
        color: `var(--status-${status})`,
        background: `var(--status-${status}-bg)`,
      }}
    >
      {Icon ? <Icon size={BADGE_ICON} strokeWidth={BADGE_STROKE} aria-hidden /> : null}
      {children}
    </span>
  );
}
