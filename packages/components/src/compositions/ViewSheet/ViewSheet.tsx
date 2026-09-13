import { useState } from "react";
import { ChevronDown, ChevronRight } from "lucide-react";
import { Button } from "../../primitives/Button/Button";
import { Sheet } from "../../primitives/Sheet/Sheet";
import { Textarea } from "../../primitives/Textarea/Textarea";
import type { DiffLine } from "../UnifiedDiff/UnifiedDiff";

/**
 * View: the code a finding or an area is about, as a chain in the order one change
 * forces the next. #904.
 *
 * **Every step starts folded to its summary**, so the story reads before the code.
 * Opening one shows its hunk, what ties it to the next step, and the whole file.
 * A note written under the chain goes onto What should change. #907.
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
  /** Adds a note for the Drone to What should change. Absent leaves the note out. */
  onAddNote?: (note: string) => void;
  /** Dismisses the finding this View is of, with the note as the reason. Absent on an area. #907. */
  onDismiss?: (reason: string) => void;
  /** The window is at `--window-floor`. */
  floor?: boolean;
  onClose?: () => void;
};

export function ViewSheet({
  open,
  title,
  steps,
  onOpenFile,
  onAddNote,
  onDismiss,
  floor = false,
  onClose,
}: ViewSheetProps) {
  const [opened, setOpened] = useState<ReadonlySet<number>>(() => new Set());
  const [draft, setDraft] = useState("");
  const [added, setAdded] = useState(0);
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
      {onAddNote === undefined ? null : (
        <div className="armada-view__note">
          <Textarea
            label="Note for the drone"
            rows={3}
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
          />
          <Button
            size="sm"
            disabled={draft.trim() === ""}
            onClick={() => {
              onAddNote(draft.trim());
              setDraft("");
              setAdded((was) => was + 1);
            }}
          >
            Add to What should change
          </Button>
          {onDismiss === undefined ? null : (
            <>
              <Button
                size="sm"
                variant="ghost"
                disabled={draft.trim() === ""}
                onClick={() => onDismiss(draft.trim())}
              >
                Dismiss this finding
              </Button>
              <p className="armada-view__tie">
                Dismissing takes the finding out of the review, keeps your note as the reason, and
                tells the next review pass not to raise it again.
              </p>
            </>
          )}
          {added === 0 ? null : (
            <p className="armada-view__tie" role="status">
              {added === 1 ? "1 note added" : `${added} notes added`} to What should change.
            </p>
          )}
        </div>
      )}
    </Sheet>
  );
}
