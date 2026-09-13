import { useState } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import { Sheet } from "../../primitives/Sheet/Sheet";
import type { DiffLine } from "../UnifiedDiff/UnifiedDiff";

/**
 * View: the code a finding or an area is about, as a chain in the order one change
 * forces the next. #904.
 *
 * **Every step starts folded to its summary**, so the story reads before the code.
 * Opening one shows its hunk, what ties it to the next step, and the whole file.
 */
export type ViewSheetStep = {
  file: string;
  summary: string;
  /** Why the next step follows. Absent on the last. */
  tieToNext?: string;
  /** The hunk as the patch holds it now, or `null` where the patch no longer holds it. */
  lines: DiffLine[] | null;
};

export type ViewSheetProps = {
  open: boolean;
  /** What the View is of: the finding, or the area's name. */
  title: string;
  steps: ViewSheetStep[];
  /** Opens the file's whole diff. Absent leaves the control out. */
  onOpenFile?: (file: string) => void;
  /** The window is at `--window-floor`. */
  floor?: boolean;
  onClose?: () => void;
};

export function ViewSheet({
  open,
  title,
  steps,
  onOpenFile,
  floor = false,
  onClose,
}: ViewSheetProps) {
  const [opened, setOpened] = useState<ReadonlySet<number>>(() => new Set());
  const allOpen = steps.length > 0 && opened.size === steps.length;
  function toggle(at: number) {
    setOpened((was) => {
      const next = new Set(was);
      if (next.has(at)) next.delete(at);
      else next.add(at);
      return next;
    });
  }
  return (
    <Sheet
      open={open}
      contained
      size="wide"
      floor={floor}
      title={title}
      subtitle={`${steps.length} ${steps.length === 1 ? "step" : "steps"}, in the order one change forces the next`}
      controls={
        <Button
          size="sm"
          variant="ghost"
          onClick={() => setOpened(allOpen ? new Set() : new Set(steps.map((_, at) => at)))}
        >
          {allOpen ? "Fold all" : "Open all"}
        </Button>
      }
      closeLabel="Close"
      closeBinding="Esc"
      onClose={onClose}
    >
      <ol className="armada-view">
        {steps.map((step, at) => {
          const isOpen = opened.has(at);
          const Mark = isOpen ? ChevronDown : ChevronRight;
          return (
            <li key={`${step.file}-${at}`} className="armada-view__step">
              <span className="armada-view__n" aria-hidden="true">
                {at + 1}
              </span>
              <div className="armada-view__body">
                <button
                  type="button"
                  className="armada-view__summary"
                  aria-expanded={isOpen}
                  onClick={() => toggle(at)}
                >
                  <span className="armada-view__what">{step.summary}</span>
                  <code className="armada-view__file">{step.file}</code>
                  <Mark size={12} aria-hidden="true" className="armada-view__mark" />
                </button>
                {/* `hidden`, not unmounted, on `DroneBrief`'s rule: a folded step stays in the page. */}
                <div className="armada-view__detail" hidden={!isOpen}>
                  {step.lines === null ? (
                    <p className="armada-view__gone">
                      This hunk is not in the patch any more. The branch moved after the review was
                      written.
                    </p>
                  ) : (
                    <pre className="armada-view__hunk">
                      {step.lines.map((line, n) => (
                        <span key={n} className="armada-view__line" data-kind={line.kind}>
                          {line.text}
                        </span>
                      ))}
                    </pre>
                  )}
                  {step.tieToNext === undefined ? null : (
                    <p className="armada-view__tie">Next: {step.tieToNext}</p>
                  )}
                  {onOpenFile === undefined ? null : (
                    <Button size="sm" variant="ghost" onClick={() => onOpenFile(step.file)}>
                      Open the whole file
                    </Button>
                  )}
                </div>
              </div>
            </li>
          );
        })}
      </ol>
    </Sheet>
  );
}
