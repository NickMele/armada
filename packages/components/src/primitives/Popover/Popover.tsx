import { useEffect, useRef, useState, type ReactNode } from "react";

/**
 * A floating layer anchored to the control that opened it. The contract names
 * popover among the surfaces that take `--bg-overlay` and among the three
 * where a shadow is legal, and says nothing else about it — no radius, no
 * padding, no width, no use. Those are read off the dropdown-menu line and
 * reported.
 *
 * Never a second floating layer inside it: elevation does not stack.
 */
export type PopoverAlign = "start" | "end";

export type PopoverProps = {
  /** The control that opens it. Renders in the normal flow. */
  trigger: ReactNode;
  children: ReactNode;
  align?: PopoverAlign;
  defaultOpen?: boolean;
};

export function Popover({ trigger, children, align = "start", defaultOpen = false }: PopoverProps) {
  const [open, setOpen] = useState(defaultOpen);
  const root = useRef<HTMLDivElement>(null);

  // Esc closes an overlay, per the global tier.
  useEffect(() => {
    if (!open) return;
    function onKey(event: KeyboardEvent) {
      if (event.key === "Escape") setOpen(false);
    }
    function onDown(event: MouseEvent) {
      if (!root.current?.contains(event.target as Node)) setOpen(false);
    }
    window.addEventListener("keydown", onKey);
    window.addEventListener("mousedown", onDown);
    return () => {
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("mousedown", onDown);
    };
  }, [open]);

  return (
    <div className="armada-popover" ref={root}>
      <span className="armada-popover__trigger" onClick={() => setOpen((v) => !v)}>
        {trigger}
      </span>
      {open ? (
        <div
          className={
            align === "end"
              ? "armada-popover__panel armada-popover__panel--end"
              : "armada-popover__panel armada-popover__panel--start"
          }
          role="dialog"
        >
          {children}
        </div>
      ) : null}
    </div>
  );
}
