import type { ReactNode } from "react";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { VerifyPanel, type VerifyPanelProps } from "../VerifyPanel/VerifyPanel";

/**
 * Verify's run, on a layer instead of above the runner — #1383. **A new home
 * rather than an edit**, `DriftSheet`'s reason and its measurement: the two
 * panels together took 45% of the window's height for an act nobody had asked
 * for yet.
 *
 * **Verify is still behind its own button.** Opening this presses nothing, and
 * the button inside starts the next run — Journey 9's rule, unchanged. The
 * steps of the last one stay readable here after it ends.
 *
 * Not `wide`: a step is a name, a command and a state.
 */
export type VerifySheetProps = VerifyPanelProps & {
  open: boolean;
  /** The file this Verify runs, in mono — `armada.yml`, or a workspace's. */
  file?: ReactNode;
  /** The window is at `--window-floor`. */
  floor?: boolean;
  onClose?: () => void;
};

export function VerifySheet({ open, file, floor = false, onClose, ...run }: VerifySheetProps) {
  return (
    <Sheet
      open={open}
      contained
      floor={floor}
      title="Verify"
      subtitle={
        file === undefined || floor ? undefined : <span className="armada-verify-sheet__file">{file}</span>
      }
      closeLabel="Close"
      closeBinding="Esc"
      bleed
      onClose={onClose}
    >
      <div className="armada-verify-sheet__body">
        <VerifyPanel {...run} titled={false} />
      </div>
    </Sheet>
  );
}
