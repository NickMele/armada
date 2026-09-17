import { useState } from "react";
import type { ReactNode } from "react";
import { ChevronDown } from "lucide-react";
import type { ButtonAnswer } from "../Button/Button";

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
  /**
   * A leading glyph on the label segment, from `packages/icons/icons.toml`
   * only. Absent draws none — most callers have no icon to lead with, since a
   * list row's label already carries the act. The title row's Dispatch is the
   * one caller today (#1107).
   */
  icon?: ReactNode;
  /** What the row could also do. Destructive last. */
  items: SplitButtonItem[];
  /**
   * Secondary on a list row, always. `primary` is legal where a surface has one
   * primary: Job detail's header, and the Board's head.
   *
   * `destructive` is outlined, never filled — a solid red control reads as an
   * error state rather than as an act. It is for a group whose every member
   * ends something, so that the caret cannot make a terminal act look like a
   * variant of the one on the face.
   *
   * `tonal` is chrome, not a list row's act: `--accent-muted` fill,
   * `--accent-hover` text, no outer border — only the inner divider between
   * segments. The title row's Dispatch is the one caller (#1087's correction
   * pass), always paired with `items={[]}` below.
   */
  variant?: "secondary" | "primary" | "destructive" | "tonal";
  /** The surface underneath: `card` fills `--bg-sunken`, `sunken` `--bg-raised`. Read by `secondary` only. */
  ground?: "card" | "sunken";
  /**
   * `default` is `--h-control` (36px), every list row and Job detail's own
   * header. `sm` is `--h-control-sm` (32px), for a control sitting beside a
   * field that height already binds — the title row's search field is the one
   * caller.
   */
  size?: "default" | "sm";
  /** Render with the menu open. Uncontrolled otherwise. */
  defaultOpen?: boolean;
  disabled?: boolean;
  /**
   * The act this control opened a dialog for is out, and Fleet has not
   * answered — the same reading as `Button`'s own `pending`, on the one
   * visible surface a split button still has once its menu has closed. The
   * face sweeps a bar and stays focusable; the caret goes off and the menu
   * will not open, since there is nothing else to disclose while a press is
   * already on its way. #1117.
   */
  pending?: boolean;
  /** Fleet's answer to the face's press, drawn as `Button`'s own `answer`. Pending wins. */
  answer?: ButtonAnswer;
  /** The face's label while `pending` holds. Falls back to `children`. */
  pendingLabel?: string;
  onAction?: () => void;
  /** Names the menu for a reader who cannot see the caret. Also the caret's own name where `items` is empty. */
  menuLabel?: string;
};

export function SplitButton({
  children,
  icon,
  items,
  variant = "secondary",
  ground = "card",
  size = "default",
  defaultOpen = false,
  disabled = false,
  pending = false,
  answer,
  pendingLabel,
  onAction,
  menuLabel = "More actions",
}: SplitButtonProps) {
  const [open, setOpen] = useState(defaultOpen);
  // Nothing behind the caret yet — the title row's Dispatch has no menu, and
  // both segments call the same handler. A menu with nothing in it is a floating
  // empty box, not an absence, so the caret has to stop opening one rather than
  // opening one with nothing to show.
  const noMenu = items.length === 0;

  return (
    <div className="armada-split-button">
      <div
        className="armada-split-button__control"
        data-variant={variant}
        data-ground={ground}
        data-size={size === "default" ? undefined : size}
      >
        <button
          type="button"
          className="armada-split-button__action"
          data-pending={pending || undefined}
          data-answer={pending ? undefined : answer}
          // Not `disabled`, on `Button`'s own reasoning: a disabled control
          // drops focus and is skipped by a screen reader, and this is the one
          // still standing for the press that is out.
          disabled={pending ? undefined : disabled}
          aria-disabled={pending || undefined}
          aria-busy={pending || undefined}
          onClick={pending ? undefined : onAction}
        >
          {icon}
          {pending ? (pendingLabel ?? children) : children}
        </button>
        <button
          type="button"
          className="armada-split-button__caret"
          aria-haspopup={noMenu ? undefined : "menu"}
          aria-expanded={noMenu ? undefined : pending ? false : open}
          aria-label={menuLabel}
          disabled={disabled || pending}
          onClick={() => (noMenu ? onAction?.() : setOpen((was) => !was))}
        >
          <ChevronDown size={16} strokeWidth={2} aria-hidden="true" />
        </button>
      </div>
      {!noMenu && open && !pending && (
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
