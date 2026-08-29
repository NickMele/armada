import { useEffect, useRef, type ReactNode } from "react";
import { TriangleAlert, X } from "lucide-react";

import { ScrollArea } from "../ScrollArea/ScrollArea";

/**
 * A floating layer, so `--bg-overlay` plus a shadow — and never a second
 * elevation stacked on top of it. There is no blur anywhere: a dialog
 * separates from the canvas by a surface step and a shadow.
 *
 * The keyboard contract is `### Safety rules for single-key actions`: Cancel
 * holds initial focus, `Enter` confirms, `Esc` cancels. Those two sentences
 * disagree with each other and this component obeys them literally — see the
 * report.
 *
 * **The layer is bounded by the window and its body is the part that gives.**
 * The dialog was a single column that grew with its content, so a
 * confirmation carrying findings ran off the top and bottom of the screen with
 * no way to reach either end — and the controls, which are the reason a dialog
 * exists, were the first thing off the bottom. Title, field and actions are
 * fixed; only `children` scrolls. That is what makes "reachable at any window
 * height" a property of the component rather than of the copy somebody wrote
 * into it.
 */
export type DialogTone = "destructive" | "neutral";

/**
 * How wide the layer is. **Two, and the second is not a bigger version of the
 * first** — `default` is the measure of a confirmation that is two sentences,
 * and `wide` is the measure of one carrying machine findings, where a citation
 * wrapping mid-expression is the thing being read badly.
 */
export type DialogWidth = "default" | "wide";

export type DialogProps = {
  open: boolean;
  /** Sentence case, and it names what happens. */
  title: string;
  /** What happens and what survives, in briefing register. The scrolling part. */
  children: ReactNode;
  /**
   * What the dialog collects before it will confirm — a reason, an
   * instruction. **Pinned under the body rather than at the end of it**: a
   * field a person has to scroll past prose to reach is a field they cannot
   * reach on a short window, and the confirm control below it is already
   * disabled until they do.
   */
  field?: ReactNode;
  /** Destructive draws the outlined red confirm. Neutral draws the accent fill. */
  tone?: DialogTone;
  /** `wide` where the body carries findings rather than sentences. */
  width?: DialogWidth;
  /** The action keeps its name through the flow: Kill here produces "Killed". */
  confirmLabel: string;
  /** Disables confirm without hiding it — a blank field it would refuse anyway. */
  confirmDisabled?: boolean;
  cancelLabel?: string;
  onConfirm?: () => void;
  onCancel?: () => void;
};

export function Dialog({
  open,
  title,
  children,
  field,
  tone = "destructive",
  width = "default",
  confirmLabel,
  confirmDisabled = false,
  cancelLabel = "Cancel",
  onConfirm,
  onCancel,
}: DialogProps) {
  const cancelRef = useRef<HTMLButtonElement>(null);

  // Cancel holds initial focus. A destructive action is never one keystroke
  // from a focused row, so the safe control is the one under the cursor.
  useEffect(() => {
    if (open) cancelRef.current?.focus();
  }, [open]);

  useEffect(() => {
    if (!open) return;
    function onKey(event: KeyboardEvent) {
      if (event.key === "Escape") {
        event.preventDefault();
        onCancel?.();
      }
      if (event.key === "Enter") {
        event.preventDefault();
        if (!confirmDisabled) onConfirm?.();
      }
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open, onCancel, onConfirm, confirmDisabled]);

  if (!open) return null;

  const destructive = tone === "destructive";

  return (
    <div className="armada-dialog-scrim">
      <div
        className="armada-dialog"
        data-width={width}
        role="dialog"
        aria-modal="true"
        aria-label={title}
      >
        {/* The head holds the glyph and the title and nothing else. The body
            used to sit in this column beside the glyph, which is why it could
            not be the only region that scrolls — it is indented to the title's
            edge instead, so the alignment survives the split. */}
        <div className="armada-dialog__head">
          {destructive ? (
            <X
              className="armada-dialog__glyph armada-dialog__glyph--destructive"
              size={16}
              strokeWidth={2}
              aria-hidden="true"
            />
          ) : (
            <TriangleAlert
              className="armada-dialog__glyph armada-dialog__glyph--escalated"
              size={16}
              strokeWidth={2}
              aria-hidden="true"
            />
          )}
          <h2 className="armada-dialog__title">{title}</h2>
        </div>
        {/* The one region that gives. `ScrollArea` rather than a local
            `overflow: auto`, because macOS hides its own scrollbar until you
            scroll and a clipped region with no sign there is more below it is
            the failure this component was reported for. */}
        <ScrollArea className="armada-dialog__body">{children}</ScrollArea>
        {field === undefined ? null : <div className="armada-dialog__field">{field}</div>}
        <div className="armada-dialog__actions">
          <button
            ref={cancelRef}
            type="button"
            className="armada-dialog__button armada-dialog__button--secondary"
            onClick={onCancel}
          >
            {cancelLabel}
          </button>
          <button
            type="button"
            className={
              destructive
                ? "armada-dialog__button armada-dialog__button--destructive"
                : "armada-dialog__button armada-dialog__button--primary"
            }
            disabled={confirmDisabled}
            onClick={onConfirm}
          >
            {confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
