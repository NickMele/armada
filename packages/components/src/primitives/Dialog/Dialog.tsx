import { useEffect, useRef, type ReactNode } from "react";
import { TriangleAlert, X } from "lucide-react";

/**
 * A floating layer, so `--bg-overlay` plus a shadow — and never a second
 * elevation stacked on top of it. There is no blur anywhere: a dialog
 * separates from the canvas by a surface step and a shadow.
 *
 * The keyboard contract is `### Safety rules for single-key actions`: Cancel
 * holds initial focus, `Enter` confirms, `Esc` cancels. Those two sentences
 * disagree with each other and this component obeys them literally — see the
 * report.
 */
export type DialogTone = "destructive" | "neutral";

export type DialogProps = {
  open: boolean;
  /** Sentence case, and it names what happens. */
  title: string;
  /** What happens and what survives, in briefing register. */
  children: ReactNode;
  /** Destructive draws the outlined red confirm. Neutral draws the accent fill. */
  tone?: DialogTone;
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
  tone = "destructive",
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
      <div className="armada-dialog" role="dialog" aria-modal="true" aria-label={title}>
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
          <div className="armada-dialog__copy">
            <h2 className="armada-dialog__title">{title}</h2>
            <div className="armada-dialog__body">{children}</div>
          </div>
        </div>
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
