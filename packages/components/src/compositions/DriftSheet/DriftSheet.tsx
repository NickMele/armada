import type { ReactNode } from "react";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { DriftPanel, type DriftPanelProps } from "../DriftPanel/DriftPanel";

/**
 * Drift's reading, on a layer instead of above the runner — #1383. **A new
 * home rather than an edit**, `JobHoldsSheet`'s precedent: every rule
 * `DriftPanel` carries is unchanged, and resident it took 196px of an 817px
 * window to say everything was current.
 *
 * **Tucked away is not dismissed.** The read still happens on opening the
 * surface, at no cost, and the control that opens this carries how many lines
 * went — a reading nobody can go back to is a reading nobody trusts, which is
 * why a toast was the wrong shape for it.
 *
 * Not `wide`: a row is a path, a verdict and a line.
 */
export type DriftSheetProps = DriftPanelProps & {
  open: boolean;
  /** The file this reading is of, in mono — `armada.yml`. */
  file?: ReactNode;
  /** The window is at `--window-floor`. */
  floor?: boolean;
  onClose?: () => void;
};

export function DriftSheet({ open, file, floor = false, onClose, ...reading }: DriftSheetProps) {
  return (
    <Sheet
      open={open}
      contained
      floor={floor}
      title="Drift"
      subtitle={
        file === undefined || floor ? undefined : <span className="armada-drift-sheet__file">{file}</span>
      }
      closeLabel="Close"
      closeBinding="Esc"
      bleed
      onClose={onClose}
    >
      <div className="armada-drift-sheet__body">
        <DriftPanel {...reading} titled={false} />
      </div>
    </Sheet>
  );
}
