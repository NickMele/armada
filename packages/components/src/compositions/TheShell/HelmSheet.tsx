import type { ReactNode } from "react";
import { Sheet } from "../../primitives/Sheet/Sheet";

/**
 * Helm's dock, folded into a sheet below the layout breakpoint. **It keeps
 * Helm's frame** (#1320): opened on a narrow window, Helm is still the same
 * place it is beside the content, not one more sheet.
 *
 * The wrapper is `display: contents`, so the scrim still lays out against
 * `.armada-shell__work`. It exists only to scope the frame in `TheShell.css`
 * to this sheet; every other sheet keeps the primitive's look.
 */
export function HelmSheet({
  open,
  title,
  binding,
  onClose,
  children,
}: {
  open: boolean;
  title: string;
  binding?: string;
  onClose: () => void;
  children: ReactNode;
}) {
  return (
    <div className="armada-shell__helm-sheet">
      <Sheet open={open} title={title} contained closeLabel="Close" closeBinding={binding} onClose={onClose}>
        {children}
      </Sheet>
    </div>
  );
}
