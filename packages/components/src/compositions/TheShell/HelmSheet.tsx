import { MessageSquare } from "lucide-react";
import type { ReactNode } from "react";
import { Sheet } from "../../primitives/Sheet/Sheet";

/**
 * Helm's dock, folded into a sheet below the layout breakpoint. **It keeps
 * Helm's frame** (#1320): opened on a narrow window, Helm is still the same
 * place it is beside the content, not one more sheet.
 *
 * The chip is the dock's own — same class, same glyph, in `Sheet`'s `leading`
 * slot. `--helm` on `--helm-muted` over the sheet's `--bg-overlay` reads
 * 4.65:1, over the 3:1 a non-text mark takes.
 *
 * The wrapper is `display: contents`, so the scrim still lays out against
 * `.armada-shell__work`. It exists only to scope the frame in `TheShell.css`
 * to this sheet; every other sheet keeps the primitive's look.
 */
export function HelmSheet({
  open,
  title,
  binding,
  controls,
  onClose,
  children,
}: {
  open: boolean;
  title: string;
  binding?: string;
  /** The dock's own act, beside the close — `Sheet`'s header slot, so a folded dock keeps it. */
  controls?: ReactNode;
  onClose: () => void;
  children: ReactNode;
}) {
  return (
    <div className="armada-shell__helm-sheet">
      <Sheet
        open={open}
        title={title}
        leading={
          <span className="armada-shell__dock-chip" aria-hidden>
            <MessageSquare size={16} strokeWidth={2} />
          </span>
        }
        contained
        controls={controls}
        closeLabel="Close"
        closeBinding={binding}
        onClose={onClose}
      >
        {children}
      </Sheet>
    </div>
  );
}
