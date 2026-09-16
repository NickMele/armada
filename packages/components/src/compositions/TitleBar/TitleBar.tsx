import { MessageSquare, Plus, Search } from "lucide-react";
import type { ReactNode } from "react";
import { ArmadaLockupHorizontal } from "@armada/brand";
import { Button } from "../../primitives/Button/Button";
import { KbdCmd } from "../../primitives/Kbd/Kbd";
import { useShortcutReveal } from "../../shortcut-reveal";

/**
 * The title row — what used to be macOS's grey bar saying only "Armada".
 * `titleBarStyle: 'hiddenInset'` insets the traffic lights over this bar
 * rather than over the sidebar; the whole row is a drag region, and every
 * control in it opts out with its own no-drag rule. #1087.
 *
 * **Three regions, not one flex row with a trailing spacer.** Left carries the
 * repository picker, centered carries search and Dispatch as a group, right
 * carries Helm's reopen control and the logo — `TitleBar.css`'s grid keeps the
 * center group on the bar's true midpoint however wide the two sides measure.
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
   * Opens the composer.
   *
   * The Board's own menu (Refresh, Reported, Held disk, Settings, the
   * two bulk acts) stays on the Board's own head — Dispatch here is a plain
   * button, not a split one. #1156: a `SplitButton` with nothing behind its
   * caret still drew a caret, so a person saw a menu that was not there.
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
  const revealing = useShortcutReveal();
  return (
    <div className="armada-title-bar">
      <div className="armada-title-bar__start">
        {repositoryPicker === undefined ? null : (
          <div className="armada-title-bar__picker">{repositoryPicker}</div>
        )}
      </div>

      <div className="armada-title-bar__center">
        {onSearch === undefined ? null : (
          <button type="button" className="armada-title-bar__search" onClick={onSearch}>
            <Search size={16} strokeWidth={2} aria-hidden />
            <span className="armada-title-bar__search-label">Search jobs, commands, settings…</span>
            <KbdCmd shortcut="⌘K" />
          </button>
        )}

        {onDispatch === undefined ? null : (
          // Wrapped, not styled directly: `Button` takes no `className` to
          // carry a title-bar-only no-drag hook on, same reasoning as the
          // picker below. #1156.
          <div className="armada-title-bar__dispatch">
            <Button variant="tonal" size="sm" onClick={onDispatch} disabled={dispatchDisabled}>
              <Plus size={16} strokeWidth={2} aria-hidden />
              Dispatch
            </Button>
          </div>
        )}
      </div>

      <div className="armada-title-bar__end">
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
            {/* Mounted only while Cmd is held, so the button shrinks to fit
                "Helm" alone at rest rather than always reserving the badge's
                width — the logo beside it shifts a few px when this mounts. */}
            {helm.binding !== undefined && revealing ? (
              <span className="armada-title-bar__helmkbd">
                <KbdCmd shortcut={helm.binding} />
              </span>
            ) : null}
          </button>
        )}

        <ArmadaLockupHorizontal height={20} title="Armada" />
      </div>
    </div>
  );
}
