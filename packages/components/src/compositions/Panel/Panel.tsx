import { ChevronRight, ChevronUp } from "lucide-react";
import type { ReactNode } from "react";

/**
 * A rounded panel in the left column — Navigation, Stats and Fleet stack
 * inside these. Bridge/1088.
 *
 * **Controlled, not persisted**: the caller reads and writes `open`, the way
 * `Sidebar`'s own `collapsed` works, so a restart can restore it.
 *
 * **Collapses to its head, never to nothing** — `trailing` stays visible
 * either way, so a glance still answers the one question the panel is for.
 *
 * `chevron-up` / `chevron-right`, `Chapter`'s own bare-toggle pair — not
 * `chevron-down`, which the mock draws by rotating one glyph in CSS.
 */
export type PanelProps = {
  label: ReactNode;
  /** Kept beside the label whether the panel is open or collapsed — a count, a dot. */
  trailing?: ReactNode;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /**
   * Below `--layout-breakpoint`. The head centres on one status dot and the
   * body never draws — the same floor the rail's own 48px form holds.
   */
  narrow?: boolean;
  /** The dot's tone at `narrow`, and nothing else — Panel carries no colour of its own. */
  dotTone?: "success" | "warn" | "escalated" | "muted";
  children: ReactNode;
};

export function Panel({
  label,
  trailing,
  open,
  onOpenChange,
  narrow = false,
  dotTone = "muted",
  children,
}: PanelProps) {
  if (narrow) {
    return (
      <section className="armada-panel" data-narrow aria-label={typeof label === "string" ? label : undefined}>
        <div className="armada-panel__head">
          <span className="armada-panel__dot" data-tone={dotTone} aria-hidden />
        </div>
      </section>
    );
  }

  return (
    <section className="armada-panel" data-open={open || undefined}>
      <button
        type="button"
        className="armada-panel__head"
        aria-expanded={open}
        onClick={() => onOpenChange(!open)}
      >
        <span className="armada-panel__label">{label}</span>
        <span className="armada-panel__trailing">
          {trailing}
          {open ? (
            <ChevronUp size={14} strokeWidth={2.2} aria-hidden />
          ) : (
            <ChevronRight size={14} strokeWidth={2.2} aria-hidden />
          )}
        </span>
      </button>
      {open ? <div className="armada-panel__body">{children}</div> : null}
    </section>
  );
}
