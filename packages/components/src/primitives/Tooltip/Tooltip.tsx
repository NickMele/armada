import { useEffect, useRef, useState, type ReactNode } from "react";

/**
 * A tooltip carries the full value of anything truncated in a row — a path, a
 * branch name, a full timestamp. It never carries an explanation the row
 * should have made plain.
 *
 * Where the action has a binding it gains a trailing kbd. The 400ms delay
 * stands, and it is a token: `--tooltip-delay`.
 */
export type TooltipProps = {
  /** The full value, or the label of the action. Sentence case. */
  label: ReactNode;
  /** The binding, where the action has one. Rendered as a trailing kbd. */
  shortcut?: string;
  children: ReactNode;
  /** Render open without hovering. Storybook draws resting states. */
  defaultOpen?: boolean;
};

export function Tooltip({ label, shortcut, children, defaultOpen = false }: TooltipProps) {
  const [open, setOpen] = useState(defaultOpen);
  const timer = useRef<number | undefined>(undefined);

  useEffect(() => () => window.clearTimeout(timer.current), []);

  function readDelay() {
    const raw = getComputedStyle(document.documentElement).getPropertyValue("--tooltip-delay");
    return Number.parseInt(raw, 10) || 0;
  }

  function show() {
    window.clearTimeout(timer.current);
    timer.current = window.setTimeout(() => setOpen(true), readDelay());
  }

  function hide() {
    window.clearTimeout(timer.current);
    setOpen(false);
  }

  return (
    <span
      className="armada-tooltip"
      onMouseEnter={show}
      onMouseLeave={hide}
      onFocus={show}
      onBlur={hide}
    >
      {children}
      {open ? (
        <span className="armada-tooltip__bubble" role="tooltip">
          <span className="armada-tooltip__label">{label}</span>
          {shortcut ? <kbd className="armada-tooltip__kbd">{shortcut}</kbd> : null}
        </span>
      ) : null}
    </span>
  );
}
