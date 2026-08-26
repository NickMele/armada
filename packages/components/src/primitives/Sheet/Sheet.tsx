import { useEffect, useRef, type ReactNode } from "react";
import { X } from "lucide-react";

/**
 * A panel that enters from an edge. The contract gives it exactly one line —
 * "Sheet and dialog use the same surface treatment at `--radius-lg`" — and the
 * component sheet draws it nowhere, so everything below the surface treatment
 * is read off Dialog and reported as underspecified.
 *
 * `x` is the shadcn dialog close, which the icon registry sanctions as chrome.
 */
export type SheetSide = "right" | "left";

export type SheetProps = {
  open: boolean;
  /** Sentence case. Panel headings may open with a Wh- word; sentences may not. */
  title: string;
  children: ReactNode;
  side?: SheetSide;
  /** The one action a sheet's footer carries, where it carries one. */
  footer?: ReactNode;
  onClose?: () => void;
};

export function Sheet({ open, title, children, side = "right", footer, onClose }: SheetProps) {
  const closeRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (open) closeRef.current?.focus();
  }, [open]);

  // Esc closes an overlay, per the global tier.
  useEffect(() => {
    if (!open) return;
    function onKey(event: KeyboardEvent) {
      if (event.key === "Escape") {
        event.preventDefault();
        onClose?.();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onClose]);

  if (!open) return null;

  return (
    <div className="armada-sheet-scrim">
      <div
        className={
          side === "right"
            ? "armada-sheet armada-sheet--right"
            : "armada-sheet armada-sheet--left"
        }
        role="dialog"
        aria-modal="true"
        aria-label={title}
      >
        <div className="armada-sheet__head">
          <h2 className="armada-sheet__title">{title}</h2>
          <button
            ref={closeRef}
            type="button"
            className="armada-sheet__close"
            aria-label="Close"
            onClick={onClose}
          >
            <X size={16} strokeWidth={2} aria-hidden="true" />
          </button>
        </div>
        <div className="armada-sheet__body">{children}</div>
        {footer ? <div className="armada-sheet__foot">{footer}</div> : null}
      </div>
    </div>
  );
}
