import { useEffect, useRef, type ReactNode } from "react";
import { X } from "lucide-react";
import { Button } from "../Button/Button";
import { Kbd } from "../Kbd/Kbd";

/**
 * A panel that enters from an edge. The contract gives it exactly one line —
 * "Sheet and dialog use the same surface treatment at `--radius-lg`" — and the
 * component sheet draws it nowhere, so everything below the surface treatment
 * is read off Dialog and reported as underspecified.
 *
 * `x` is the shadcn dialog close, which the icon registry sanctions as chrome.
 *
 * # The trailing sheet, and why the parts are slots
 *
 * Journey 4's frames `4i`–`4m` put two readings on this layer — a step's
 * activity log and the Job's whole patch — and neither is a longer version of
 * something a panel can hold: 1676 entries pushes the rest of the screen off
 * the bottom and a patch in a 602px column is unreadable. The frames draw four
 * parts this component had no slot for, so each is a slot rather than a second
 * component: a subtitle under the title, controls in the header, full-width
 * bands under it, and a body that carries its own padding.
 *
 * **Two exits and no third.** The labelled control and `Esc`. A click on the
 * ground behind does not close a sheet — a 1676-entry read must not be
 * dismissed by a stray click, so the scrim takes no press.
 *
 * **`Esc` is caught in the capture phase and stopped there.** The registry row
 * reads *closes an overlay, or returns to the list from a detail route*, and
 * both clauses are bound on `window`: without the stop, one press would close
 * the sheet and leave the Job at the same time.
 */
export type SheetSide = "right" | "left";

/**
 * How much of the ground the sheet takes. The drawing measures both as a
 * fraction rather than a width, because what has to fit is the reading and not
 * a column: `wide` is `4i`'s 62% and `widest` is `4j`'s 76%, which is the file
 * rail plus a patch line that does not wrap.
 *
 * `default` is the sheet the component sheet already drew, at `--w-sheet`.
 *
 * MISSING TOKEN, reported: `--w-sheet` is 480px and describes neither of these.
 * A fraction of the ground is not a width and has no token to be.
 */
export type SheetSize = "default" | "wide" | "widest";

export type SheetProps = {
  open: boolean;
  /** Sentence case. Panel headings may open with a Wh- word; sentences may not. */
  title: string;
  /**
   * The line under the title — what this sheet is of, and how much of it.
   * `Fix · job_2d90bb · 1676 entries · live`.
   */
  subtitle?: ReactNode;
  children: ReactNode;
  side?: SheetSide;
  size?: SheetSize;
  /**
   * Controls in the header, between the titles and the close — the log's four
   * filters. `tabs` on a layer is a departure the drawing states: nothing else
   * places the primitive on one, and four filters over 1676 entries are three
   * hidden behind a select.
   */
  controls?: ReactNode;
  /**
   * Full-width bands between the header and the body — the held strip, the
   * escalation notice. They are bands rather than body content because they
   * stay while the body scrolls under them.
   */
  bands?: ReactNode;
  /** The body carries its own padding, for a reading that runs edge to edge. */
  bleed?: boolean;
  /**
   * Laid out inside the nearest positioned ancestor rather than over the
   * window. **What a trailing sheet takes**: the layer belongs to the screen it
   * was opened from, and a window-fixed one would cover the shell's rail as
   * well, which nothing asked it to.
   */
  contained?: boolean;
  /**
   * The close control's label, and the binding drawn beside it. Absent leaves
   * the close icon-only with the binding in its tooltip, which is what the
   * floor takes — `4l`, and there only.
   */
  closeLabel?: string;
  closeBinding?: string;
  /**
   * The window is at `--window-floor`. Flush to both edges, no radius, and the
   * close goes icon-only. **A prop rather than a media query**: a media query
   * cannot read a custom property, and the Tailwind breakpoint variants that
   * were meant to stand in for one emit `@media (width >= var(--layout-breakpoint))`,
   * which every browser drops. The app reads `--window-floor` itself and passes
   * the answer down. Reported.
   */
  floor?: boolean;
  /** The one action a sheet's footer carries, where it carries one. */
  footer?: ReactNode;
  onClose?: () => void;
};

export function Sheet({
  open,
  title,
  subtitle,
  children,
  side = "right",
  size = "default",
  controls,
  bands,
  bleed = false,
  contained = false,
  closeLabel,
  closeBinding,
  floor = false,
  footer,
  onClose,
}: SheetProps) {
  const closeRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (open) closeRef.current?.focus();
  }, [open]);

  // Esc closes an overlay, per the global tier — and stops there. Bound in the
  // capture phase because the other clause of the same registry row, "returns
  // to the list from a detail route", is bound on `window` too: a bubble-phase
  // listener would run second and both would answer one press.
  useEffect(() => {
    if (!open) return;
    function onKey(event: KeyboardEvent) {
      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        onClose?.();
      }
    }
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, [open, onClose]);

  if (!open) return null;

  const labelled = closeLabel !== undefined && !floor;
  const tooltip = closeBinding === undefined ? "Close" : `Close — ${closeBinding}`;

  return (
    <div className="armada-sheet-scrim" data-contained={contained || undefined}>
      <div
        className="armada-sheet"
        data-side={side}
        data-size={size}
        data-floor={floor || undefined}
        role="dialog"
        aria-modal="true"
        aria-label={title}
      >
        <div className="armada-sheet__head">
          <div className="armada-sheet__titles">
            <h2 className="armada-sheet__title" data-titled={subtitle !== undefined || undefined}>
              {title}
            </h2>
            {subtitle === undefined ? null : (
              <span className="armada-sheet__subtitle">{subtitle}</span>
            )}
          </div>
          {controls === undefined ? null : (
            <div className="armada-sheet__controls">{controls}</div>
          )}
          {labelled ? (
            /* A secondary on an overlay is filled one surface step from its
               ground, which is what `ground="sunken"` spells. */
            <Button
              ref={closeRef}
              variant="secondary"
              size="sm"
              ground="sunken"
              title={tooltip}
              onClick={onClose}
            >
              {closeLabel}
              {closeBinding === undefined ? null : <Kbd>{closeBinding}</Kbd>}
            </Button>
          ) : (
            <button
              ref={closeRef}
              type="button"
              className="armada-sheet__close"
              aria-label="Close"
              title={tooltip}
              onClick={onClose}
            >
              <X size={16} strokeWidth={2} aria-hidden="true" />
            </button>
          )}
        </div>
        {bands === undefined ? null : <div className="armada-sheet__bands">{bands}</div>}
        <div className="armada-sheet__body" data-bleed={bleed || undefined}>
          {children}
        </div>
        {footer ? <div className="armada-sheet__foot">{footer}</div> : null}
      </div>
    </div>
  );
}
