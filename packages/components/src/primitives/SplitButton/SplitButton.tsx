import { useState } from "react";
import { ChevronDown } from "lucide-react";

/**
 * The likely action, with the rest one click away — a button and a dropdown
 * menu in one control, not a new primitive.
 *
 * The label is always the action a person is most likely to take from that
 * state, so it changes with the Job, and it never repeats inside the menu. A
 * split button with nothing in its menu is a button.
 *
 * The caret is not a label icon. It is the whole content of its own divided
 * segment, structural rather than decorative — the one exception to
 * label-only buttons, and the only thing `chevron-down` is granted here.
 *
 * A row carries this or an ellipsis, never both: the ellipsis means there is
 * no likely action, this means there is and it is the label.
 */
export type SplitButtonItem = {
  /** The verb, sentence case. Never a repeat of the label, and never "Open". */
  label: string;
  /** The single-key or modifier binding, if the action has one. */
  shortcut?: string;
  /** Destructive. Last in the list, `--status-completed-failed` text. */
  danger?: boolean;
  onSelect?: () => void;
};

export type SplitButtonProps = {
  /** The act the state calls for. */
  children: string;
  /** What the row could also do. Destructive last. */
  items: SplitButtonItem[];
  /**
   * Secondary on a list row, always. `primary` is legal only on job detail,
   * where there is one Job and one primary.
   *
   * `destructive` is outlined, never filled — a solid red control reads as an
   * error state rather than as an act. It is for a group whose every member
   * ends something, so that the caret cannot make a terminal act look like a
   * variant of the one on the face.
   */
  variant?: "secondary" | "primary" | "destructive";
  /** The surface underneath: `card` fills `--bg-sunken`, `sunken` `--bg-raised`. */
  ground?: "card" | "sunken";
  /** Render with the menu open. Uncontrolled otherwise. */
  defaultOpen?: boolean;
  disabled?: boolean;
  onAction?: () => void;
  /** Names the menu for a reader who cannot see the caret. */
  menuLabel?: string;
};

export function SplitButton({
  children,
  items,
  variant = "secondary",
  ground = "card",
  defaultOpen = false,
  disabled = false,
  onAction,
  menuLabel = "More actions",
}: SplitButtonProps) {
  const [open, setOpen] = useState(defaultOpen);

  return (
    <div className="armada-split-button">
      <div className="armada-split-button__control" data-variant={variant} data-ground={ground}>
        <button
          type="button"
          className="armada-split-button__action"
          disabled={disabled}
          onClick={onAction}
        >
          {children}
        </button>
        <button
          type="button"
          className="armada-split-button__caret"
          aria-haspopup="menu"
          aria-expanded={open}
          aria-label={menuLabel}
          disabled={disabled}
          onClick={() => setOpen((was) => !was)}
        >
          <ChevronDown size={16} strokeWidth={2} aria-hidden="true" />
        </button>
      </div>
      {open && (
        <div className="armada-split-button__menu" role="menu" aria-label={menuLabel}>
          {items.map((item) => (
            <button
              key={item.label}
              type="button"
              role="menuitem"
              className="armada-split-button__item"
              data-danger={item.danger || undefined}
              // Closes on choosing. A menu still open over the confirmation it
              // just raised is a control that did not respond.
              onClick={() => {
                setOpen(false);
                item.onSelect?.();
              }}
            >
              <span>{item.label}</span>
              {item.shortcut !== undefined && (
                <span className="armada-split-button__shortcut">{item.shortcut}</span>
              )}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
