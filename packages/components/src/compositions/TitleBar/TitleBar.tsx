import { ChevronDown, MessageSquare, Search } from "lucide-react";
import type { ReactNode } from "react";
import { ArmadaLockupHorizontal } from "@armada/brand";
import { Kbd } from "../../primitives/Kbd/Kbd";

/**
 * The title row — what used to be macOS's grey bar saying only "Armada".
 * `titleBarStyle: 'hiddenInset'` insets the traffic lights over this bar
 * rather than over the sidebar; the whole row is a drag region, and every
 * control in it opts out with its own no-drag rule. #1087.
 *
 * **Left to right, the owner's own order**: the repository picker (moved out
 * of the rail), the search field that opens the palette, Dispatch, a spacer,
 * Helm's own reopen control while its dock is closed, and the logo.
 *
 * **A slot, not a decision.** `repositoryPicker` arrives built: the Select and
 * its options are Bridge's own reading of what Fleet serves, and this row only
 * has room for it, the way `TheShell`'s `railHeader` used to.
 */
export type TitleBarProps = {
  /** The repository picker. Absent draws none — a bare bar until Bridge has one to offer. */
  repositoryPicker?: ReactNode;
  /** Opens the command palette. The same surface ⌘K opens; this is the other way in. */
  onSearch?: () => void;
  /**
   * Opens the composer. **Both segments of Dispatch call this.**
   *
   * The drawing splits a label from a caret, but nothing here belongs behind
   * one yet — the Board's own menu (Refresh, Reported, Held disk, Fleet
   * settings, the two bulk acts) stays on the Board's own head, and Fleet
   * settings gets a screen of its own in #1089. A caret with nothing under it
   * is still drawn, matching the mock, and it opens the same form the label
   * does rather than a menu with nothing in it.
   */
  onDispatch?: () => void;
  /** Disabled while nothing is connected to dispatch into. */
  dispatchDisabled?: boolean;
  /** Helm's reopen control. Absent while the dock is already open beside the content. */
  helm?: {
    /** Waiting on a person, across every repository. Zero draws no count. */
    questions: number;
    /** ⌘J, beside the label. */
    binding?: string;
    onOpen: () => void;
  };
};

export function TitleBar({
  repositoryPicker,
  onSearch,
  onDispatch,
  dispatchDisabled = false,
  helm,
}: TitleBarProps) {
  return (
    <div className="armada-title-bar">
      {repositoryPicker === undefined ? null : (
        <div className="armada-title-bar__picker">{repositoryPicker}</div>
      )}

      {onSearch === undefined ? null : (
        <button type="button" className="armada-title-bar__search" onClick={onSearch}>
          <Search size={16} strokeWidth={2} aria-hidden />
          <span className="armada-title-bar__search-label">Search jobs, commands, settings…</span>
          <Kbd>⌘K</Kbd>
        </button>
      )}

      {onDispatch === undefined ? null : (
        <div className="armada-title-bar__dispatch">
          <button
            type="button"
            className="armada-title-bar__dispatch-action"
            disabled={dispatchDisabled}
            onClick={onDispatch}
          >
            Dispatch
          </button>
          <button
            type="button"
            className="armada-title-bar__dispatch-caret"
            aria-label="Dispatch a job"
            disabled={dispatchDisabled}
            onClick={onDispatch}
          >
            <ChevronDown size={16} strokeWidth={2} aria-hidden />
          </button>
        </div>
      )}

      <div className="armada-title-bar__spacer" />

      {helm === undefined ? null : (
        <button
          type="button"
          className="armada-title-bar__helm"
          onClick={helm.onOpen}
          title={helm.binding === undefined ? "Open Helm" : `Open Helm — ${helm.binding}`}
        >
          <MessageSquare size={16} strokeWidth={2} aria-hidden />
          <span>Helm</span>
          {helm.questions > 0 ? (
            <span className="armada-title-bar__helm-count">{helm.questions}</span>
          ) : null}
        </button>
      )}

      <ArmadaLockupHorizontal height={20} title="Armada" />
    </div>
  );
}
